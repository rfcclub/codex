use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use futures::StreamExt;
use serde_json::Value;

use crate::AdmissionController;
use crate::AdmissionError;
use crate::NativeRoute;
use crate::NativeSettings;
use crate::UpstreamProtocol;
use crate::admission::AdmissionLease;
use crate::translation::AnthropicSseToChat;
use crate::translation::AnthropicSseToResponses;
use crate::translation::ChatSseToAnthropic;
use crate::translation::ChatSseToResponses;
use crate::translation::ResponsesSseToAnthropic;
use crate::translation::ResponsesSseToChat;
use crate::translation::anthropic_message_to_chat_completion;
use crate::translation::anthropic_message_to_chat_completion_request;
use crate::translation::anthropic_message_to_response;
use crate::translation::anthropic_message_to_responses_request;
use crate::translation::chat_completion_to_anthropic_message;
use crate::translation::chat_completion_to_anthropic_message_response;
use crate::translation::chat_completion_to_response;
use crate::translation::chat_completion_to_responses_request;
use crate::translation::response_to_chat_completion;
use crate::translation::responses_to_anthropic_message;
use crate::translation::responses_to_anthropic_message_response;
use crate::translation::responses_to_chat_completion;

/// Loopback HTTP surface for native model providers.
#[derive(Clone)]
pub struct NativeShimServer {
    settings: Arc<NativeSettings>,
    admission: AdmissionController,
    client: reqwest::Client,
}

/// A bound loopback server owned by the shim launcher.
pub struct NativeShimRuntime {
    base_url: String,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

impl NativeShimRuntime {
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn shutdown(mut self) -> Result<(), tokio::task::JoinError> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        self.task.await
    }
}

impl NativeShimServer {
    pub fn new(settings: Arc<NativeSettings>) -> Result<Self, AdmissionError> {
        Ok(Self {
            admission: AdmissionController::from_settings(&settings)?,
            settings,
            client: reqwest::Client::new(),
        })
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/health", get(health))
            .route("/healthz", get(health))
            .route("/diagnostics", get(diagnostics))
            .route("/v1/responses", post(responses))
            .route("/v1/chat/completions", post(chat_completions))
            .route("/v1/messages", post(messages))
            .with_state(self)
    }

    pub async fn start(self, port: Option<u16>) -> Result<NativeShimRuntime, std::io::Error> {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port.unwrap_or(0))).await?;
        let address = listener.local_addr()?;
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, self.router())
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await;
        });
        Ok(NativeShimRuntime {
            base_url: format!("http://{address}"),
            shutdown_tx: Some(shutdown_tx),
            task,
        })
    }
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn diagnostics(State(server): State<NativeShimServer>) -> impl IntoResponse {
    let routes = server
        .settings
        .routes()
        .iter()
        .map(|route| {
            let capacity = server.admission.diagnostics(route.provider_id());
            serde_json::json!({
                "slug": route.slug(),
                "provider_id": route.provider_id(),
                "active": capacity.as_ref().map(|capacity| capacity.active),
                "max_active": capacity.as_ref().map(|capacity| capacity.max_active),
                "queued": capacity.as_ref().map(|capacity| capacity.queued),
                "max_queued": capacity.as_ref().map(|capacity| capacity.max_queued),
            })
        })
        .collect::<Vec<_>>();
    Json(serde_json::json!({ "routes": routes }))
}

async fn responses(
    State(server): State<NativeShimServer>,
    Json(mut body): Json<Value>,
) -> Response {
    let Some(model) = body.get("model").and_then(Value::as_str) else {
        return shim_error(StatusCode::BAD_REQUEST, "request is missing a string model");
    };
    let Some(route) = server
        .settings
        .routes()
        .iter()
        .find(|route| route.slug() == model)
    else {
        return shim_error(StatusCode::NOT_FOUND, "requested model is not configured");
    };
    match route.protocol() {
        UpstreamProtocol::OpenAiResponses => {
            body["model"] = Value::String(route.model().to_owned());
            let lease = match server.admission.acquire(route.provider_id()).await {
                Ok(lease) => lease,
                Err(error) => return admission_error(error),
            };
            forward_request_to_upstream(&server.client, route, body, "responses", false, lease)
                .await
        }
        UpstreamProtocol::OpenAiChatCompletions => {
            let translated = match responses_to_chat_completion(body, route.model()) {
                Ok(translated) => translated,
                Err(error) => return shim_error(StatusCode::BAD_REQUEST, &error.to_string()),
            };
            let lease = match server.admission.acquire(route.provider_id()).await {
                Ok(lease) => lease,
                Err(error) => return admission_error(error),
            };
            forward_chat_as_response(&server.client, route, translated, lease).await
        }
        UpstreamProtocol::AnthropicMessages => {
            let translated = match responses_to_anthropic_message(body, route.model()) {
                Ok(translated) => translated,
                Err(error) => return shim_error(StatusCode::BAD_REQUEST, &error.to_string()),
            };
            let lease = match server.admission.acquire(route.provider_id()).await {
                Ok(lease) => lease,
                Err(error) => return admission_error(error),
            };
            forward_anthropic_as_response(&server.client, route, translated, lease).await
        }
    }
}

async fn chat_completions(
    State(server): State<NativeShimServer>,
    Json(mut body): Json<Value>,
) -> Response {
    let Some(model) = body.get("model").and_then(Value::as_str) else {
        return shim_error(StatusCode::BAD_REQUEST, "request is missing a string model");
    };
    let Some(route) = server
        .settings
        .routes()
        .iter()
        .find(|route| route.slug() == model)
    else {
        return shim_error(StatusCode::NOT_FOUND, "requested model is not configured");
    };
    match route.protocol() {
        UpstreamProtocol::OpenAiChatCompletions => {
            body["model"] = Value::String(route.model().to_owned());
            let lease = match server.admission.acquire(route.provider_id()).await {
                Ok(lease) => lease,
                Err(error) => return admission_error(error),
            };
            forward_request_to_upstream(
                &server.client,
                route,
                body,
                "chat/completions",
                false,
                lease,
            )
            .await
        }
        UpstreamProtocol::AnthropicMessages => {
            let translated = match chat_completion_to_anthropic_message(body, route.model()) {
                Ok(translated) => translated,
                Err(error) => return shim_error(StatusCode::BAD_REQUEST, &error.to_string()),
            };
            let lease = match server.admission.acquire(route.provider_id()).await {
                Ok(lease) => lease,
                Err(error) => return admission_error(error),
            };
            forward_anthropic_as_chat(&server.client, route, translated, lease).await
        }
        UpstreamProtocol::OpenAiResponses => {
            let translated = match chat_completion_to_responses_request(body, route.model()) {
                Ok(translated) => translated,
                Err(error) => return shim_error(StatusCode::BAD_REQUEST, &error.to_string()),
            };
            let lease = match server.admission.acquire(route.provider_id()).await {
                Ok(lease) => lease,
                Err(error) => return admission_error(error),
            };
            forward_responses_as_chat(&server.client, route, translated, lease).await
        }
    }
}

async fn messages(State(server): State<NativeShimServer>, Json(mut body): Json<Value>) -> Response {
    let Some(model) = body.get("model").and_then(Value::as_str) else {
        return shim_error(StatusCode::BAD_REQUEST, "request is missing a string model");
    };
    let Some(route) = server
        .settings
        .routes()
        .iter()
        .find(|route| route.slug() == model)
    else {
        return shim_error(StatusCode::NOT_FOUND, "requested model is not configured");
    };
    match route.protocol() {
        UpstreamProtocol::AnthropicMessages => {
            body["model"] = Value::String(route.model().to_owned());
            let lease = match server.admission.acquire(route.provider_id()).await {
                Ok(lease) => lease,
                Err(error) => return admission_error(error),
            };
            forward_request_to_upstream(&server.client, route, body, "messages", true, lease).await
        }
        UpstreamProtocol::OpenAiResponses => {
            let translated = match anthropic_message_to_responses_request(body, route.model()) {
                Ok(translated) => translated,
                Err(error) => return shim_error(StatusCode::BAD_REQUEST, &error.to_string()),
            };
            let lease = match server.admission.acquire(route.provider_id()).await {
                Ok(lease) => lease,
                Err(error) => return admission_error(error),
            };
            forward_responses_as_anthropic(&server.client, route, translated, lease).await
        }
        UpstreamProtocol::OpenAiChatCompletions => {
            let translated = match anthropic_message_to_chat_completion_request(body, route.model())
            {
                Ok(translated) => translated,
                Err(error) => return shim_error(StatusCode::BAD_REQUEST, &error.to_string()),
            };
            let lease = match server.admission.acquire(route.provider_id()).await {
                Ok(lease) => lease,
                Err(error) => return admission_error(error),
            };
            forward_chat_as_anthropic(&server.client, route, translated, lease).await
        }
    }
}

async fn forward_request_to_upstream(
    client: &reqwest::Client,
    route: &NativeRoute,
    body: Value,
    upstream_path: &str,
    use_anthropic_auth: bool,
    lease: AdmissionLease,
) -> Response {
    let mut url = route.base_url().clone();
    let path = format!("{}/{}", url.path().trim_end_matches('/'), upstream_path);
    url.set_path(&path);
    let request = if use_anthropic_auth {
        anthropic_post(client, url, route, body)
    } else {
        client.post(url).bearer_auth(route.api_key()).json(&body)
    };
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream request failed: {error}"),
            );
        }
    };
    let status = response.status();
    let mut headers = HeaderMap::new();
    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE)
        && let Ok(content_type) = HeaderValue::from_bytes(content_type.as_bytes())
    {
        headers.insert(axum::http::header::CONTENT_TYPE, content_type);
    }
    let stream = response.bytes_stream().map(move |result| {
        let _lease = &lease;
        result
    });
    (status, headers, Body::from_stream(stream)).into_response()
}

fn anthropic_post(
    client: &reqwest::Client,
    url: reqwest::Url,
    route: &NativeRoute,
    body: Value,
) -> reqwest::RequestBuilder {
    client
        .post(url)
        .header("x-api-key", route.api_key())
        .header("anthropic-version", "2023-06-01")
        .json(&body)
}

fn chat_sse_as_responses(response: reqwest::Response, lease: AdmissionLease) -> Response {
    let upstream: Pin<
        Box<dyn futures::Stream<Item = Result<axum::body::Bytes, reqwest::Error>> + Send>,
    > = Box::pin(response.bytes_stream());
    let stream = futures::stream::unfold(
        (
            upstream,
            ChatSseToResponses::default(),
            VecDeque::new(),
            lease,
        ),
        |(mut upstream, mut adapter, mut pending, lease)| async move {
            loop {
                if let Some(event) = pending.pop_front() {
                    return Some((Ok(event), (upstream, adapter, pending, lease)));
                }
                match upstream.as_mut().next().await {
                    Some(Ok(chunk)) => {
                        pending.extend(
                            adapter
                                .push_bytes(&chunk)
                                .into_iter()
                                .map(axum::body::Bytes::from),
                        );
                    }
                    Some(Err(error)) => {
                        return Some((Err(error), (upstream, adapter, pending, lease)));
                    }
                    None => return None,
                }
            }
        },
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    (StatusCode::OK, headers, Body::from_stream(stream)).into_response()
}

fn anthropic_sse_as_responses(response: reqwest::Response, lease: AdmissionLease) -> Response {
    let upstream: Pin<
        Box<dyn futures::Stream<Item = Result<axum::body::Bytes, reqwest::Error>> + Send>,
    > = Box::pin(response.bytes_stream());
    let stream = futures::stream::unfold(
        (
            upstream,
            AnthropicSseToResponses::default(),
            VecDeque::new(),
            lease,
        ),
        |(mut upstream, mut adapter, mut pending, lease)| async move {
            loop {
                if let Some(event) = pending.pop_front() {
                    return Some((Ok(event), (upstream, adapter, pending, lease)));
                }
                match upstream.as_mut().next().await {
                    Some(Ok(chunk)) => {
                        pending.extend(
                            adapter
                                .push_bytes(&chunk)
                                .into_iter()
                                .map(axum::body::Bytes::from),
                        );
                    }
                    Some(Err(error)) => {
                        return Some((Err(error), (upstream, adapter, pending, lease)));
                    }
                    None => return None,
                }
            }
        },
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    (StatusCode::OK, headers, Body::from_stream(stream)).into_response()
}

fn responses_sse_as_anthropic(response: reqwest::Response, lease: AdmissionLease) -> Response {
    let upstream: Pin<
        Box<dyn futures::Stream<Item = Result<axum::body::Bytes, reqwest::Error>> + Send>,
    > = Box::pin(response.bytes_stream());
    let stream = futures::stream::unfold(
        (
            upstream,
            ResponsesSseToAnthropic::default(),
            VecDeque::new(),
            lease,
        ),
        |(mut upstream, mut adapter, mut pending, lease)| async move {
            loop {
                if let Some(event) = pending.pop_front() {
                    return Some((Ok(event), (upstream, adapter, pending, lease)));
                }
                match upstream.as_mut().next().await {
                    Some(Ok(chunk)) => {
                        pending.extend(
                            adapter
                                .push_bytes(&chunk)
                                .into_iter()
                                .map(axum::body::Bytes::from),
                        );
                    }
                    Some(Err(error)) => {
                        return Some((Err(error), (upstream, adapter, pending, lease)));
                    }
                    None => return None,
                }
            }
        },
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    (StatusCode::OK, headers, Body::from_stream(stream)).into_response()
}

fn anthropic_sse_as_chat(response: reqwest::Response, lease: AdmissionLease) -> Response {
    let upstream: Pin<
        Box<dyn futures::Stream<Item = Result<axum::body::Bytes, reqwest::Error>> + Send>,
    > = Box::pin(response.bytes_stream());
    let stream = futures::stream::unfold(
        (
            upstream,
            AnthropicSseToChat::default(),
            VecDeque::new(),
            lease,
        ),
        |(mut upstream, mut adapter, mut pending, lease)| async move {
            loop {
                if let Some(event) = pending.pop_front() {
                    return Some((Ok(event), (upstream, adapter, pending, lease)));
                }
                match upstream.as_mut().next().await {
                    Some(Ok(chunk)) => {
                        pending.extend(
                            adapter
                                .push_bytes(&chunk)
                                .into_iter()
                                .map(axum::body::Bytes::from),
                        );
                    }
                    Some(Err(error)) => {
                        return Some((Err(error), (upstream, adapter, pending, lease)));
                    }
                    None => return None,
                }
            }
        },
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    (StatusCode::OK, headers, Body::from_stream(stream)).into_response()
}

fn responses_sse_as_chat(response: reqwest::Response, lease: AdmissionLease) -> Response {
    let upstream: Pin<
        Box<dyn futures::Stream<Item = Result<axum::body::Bytes, reqwest::Error>> + Send>,
    > = Box::pin(response.bytes_stream());
    let stream = futures::stream::unfold(
        (
            upstream,
            ResponsesSseToChat::default(),
            VecDeque::new(),
            lease,
        ),
        |(mut upstream, mut adapter, mut pending, lease)| async move {
            loop {
                if let Some(event) = pending.pop_front() {
                    return Some((Ok(event), (upstream, adapter, pending, lease)));
                }
                match upstream.as_mut().next().await {
                    Some(Ok(chunk)) => {
                        pending.extend(
                            adapter
                                .push_bytes(&chunk)
                                .into_iter()
                                .map(axum::body::Bytes::from),
                        );
                    }
                    Some(Err(error)) => {
                        return Some((Err(error), (upstream, adapter, pending, lease)));
                    }
                    None => return None,
                }
            }
        },
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    (StatusCode::OK, headers, Body::from_stream(stream)).into_response()
}

fn chat_sse_as_anthropic(response: reqwest::Response, lease: AdmissionLease) -> Response {
    let upstream: Pin<
        Box<dyn futures::Stream<Item = Result<axum::body::Bytes, reqwest::Error>> + Send>,
    > = Box::pin(response.bytes_stream());
    let stream = futures::stream::unfold(
        (
            upstream,
            ChatSseToAnthropic::default(),
            VecDeque::new(),
            lease,
        ),
        |(mut upstream, mut adapter, mut pending, lease)| async move {
            loop {
                if let Some(event) = pending.pop_front() {
                    return Some((Ok(event), (upstream, adapter, pending, lease)));
                }
                match upstream.as_mut().next().await {
                    Some(Ok(chunk)) => {
                        pending.extend(
                            adapter
                                .push_bytes(&chunk)
                                .into_iter()
                                .map(axum::body::Bytes::from),
                        );
                    }
                    Some(Err(error)) => {
                        return Some((Err(error), (upstream, adapter, pending, lease)));
                    }
                    None => return None,
                }
            }
        },
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    (StatusCode::OK, headers, Body::from_stream(stream)).into_response()
}

async fn forward_non_streaming_error(response: reqwest::Response) -> Response {
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "upstream request failed".to_owned());
    (status, body).into_response()
}

async fn forward_chat_as_response(
    client: &reqwest::Client,
    route: &NativeRoute,
    body: Value,
    _lease: AdmissionLease,
) -> Response {
    let streaming = body.get("stream").and_then(Value::as_bool) == Some(true);
    let mut url = route.base_url().clone();
    let path = format!("{}/chat/completions", url.path().trim_end_matches('/'));
    url.set_path(&path);
    let response = match client
        .post(url)
        .bearer_auth(route.api_key())
        .json(&body)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream request failed: {error}"),
            );
        }
    };
    if streaming {
        if !response.status().is_success() {
            return forward_non_streaming_error(response).await;
        }
        return chat_sse_as_responses(response, _lease);
    }
    let status = response.status();
    let completion = match response.json::<Value>().await {
        Ok(completion) => completion,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream response was not JSON: {error}"),
            );
        }
    };
    if !status.is_success() {
        return (status, Json(completion)).into_response();
    }
    match chat_completion_to_response(completion) {
        Ok(response) => Json(response).into_response(),
        Err(error) => shim_error(StatusCode::BAD_GATEWAY, &error.to_string()),
    }
}

async fn forward_chat_as_anthropic(
    client: &reqwest::Client,
    route: &NativeRoute,
    body: Value,
    lease: AdmissionLease,
) -> Response {
    let streaming = body.get("stream").and_then(Value::as_bool) == Some(true);
    let mut url = route.base_url().clone();
    let path = format!("{}/chat/completions", url.path().trim_end_matches('/'));
    url.set_path(&path);
    let response = match client
        .post(url)
        .bearer_auth(route.api_key())
        .json(&body)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream request failed: {error}"),
            );
        }
    };
    let status = response.status();
    if streaming {
        if !status.is_success() {
            return forward_non_streaming_error(response).await;
        }
        return chat_sse_as_anthropic(response, lease);
    }
    let completion = match response.json::<Value>().await {
        Ok(completion) => completion,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream response was not JSON: {error}"),
            );
        }
    };
    if !status.is_success() {
        return (status, Json(completion)).into_response();
    }
    match chat_completion_to_anthropic_message_response(completion) {
        Ok(message) => Json(message).into_response(),
        Err(error) => shim_error(StatusCode::BAD_GATEWAY, &error.to_string()),
    }
}

async fn forward_anthropic_as_response(
    client: &reqwest::Client,
    route: &NativeRoute,
    body: Value,
    lease: AdmissionLease,
) -> Response {
    let streaming = body.get("stream").and_then(Value::as_bool) == Some(true);
    let mut url = route.base_url().clone();
    let path = format!("{}/messages", url.path().trim_end_matches('/'));
    url.set_path(&path);
    let response = match anthropic_post(client, url, route, body).send().await {
        Ok(response) => response,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream request failed: {error}"),
            );
        }
    };
    let status = response.status();
    if streaming {
        if !status.is_success() {
            return forward_non_streaming_error(response).await;
        }
        return anthropic_sse_as_responses(response, lease);
    }
    let message = match response.json::<Value>().await {
        Ok(message) => message,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream response was not JSON: {error}"),
            );
        }
    };
    if !status.is_success() {
        return (status, Json(message)).into_response();
    }
    match anthropic_message_to_response(message) {
        Ok(response) => Json(response).into_response(),
        Err(error) => shim_error(StatusCode::BAD_GATEWAY, &error.to_string()),
    }
}

async fn forward_anthropic_as_chat(
    client: &reqwest::Client,
    route: &NativeRoute,
    body: Value,
    lease: AdmissionLease,
) -> Response {
    let streaming = body.get("stream").and_then(Value::as_bool) == Some(true);
    let mut url = route.base_url().clone();
    let path = format!("{}/messages", url.path().trim_end_matches('/'));
    url.set_path(&path);
    let response = match anthropic_post(client, url, route, body).send().await {
        Ok(response) => response,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream request failed: {error}"),
            );
        }
    };
    let status = response.status();
    if streaming {
        if !status.is_success() {
            return forward_non_streaming_error(response).await;
        }
        return anthropic_sse_as_chat(response, lease);
    }
    let message = match response.json::<Value>().await {
        Ok(message) => message,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream response was not JSON: {error}"),
            );
        }
    };
    if !status.is_success() {
        return (status, Json(message)).into_response();
    }
    match anthropic_message_to_chat_completion(message, route.model()) {
        Ok(completion) => Json(completion).into_response(),
        Err(error) => shim_error(StatusCode::BAD_GATEWAY, &error.to_string()),
    }
}

async fn forward_responses_as_chat(
    client: &reqwest::Client,
    route: &NativeRoute,
    body: Value,
    lease: AdmissionLease,
) -> Response {
    let streaming = body.get("stream").and_then(Value::as_bool) == Some(true);
    let mut url = route.base_url().clone();
    let path = format!("{}/responses", url.path().trim_end_matches('/'));
    url.set_path(&path);
    let response = match client
        .post(url)
        .bearer_auth(route.api_key())
        .json(&body)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream request failed: {error}"),
            );
        }
    };
    let status = response.status();
    if streaming {
        if !status.is_success() {
            return forward_non_streaming_error(response).await;
        }
        return responses_sse_as_chat(response, lease);
    }
    let response = match response.json::<Value>().await {
        Ok(response) => response,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream response was not JSON: {error}"),
            );
        }
    };
    if !status.is_success() {
        return (status, Json(response)).into_response();
    }
    match response_to_chat_completion(response, route.model()) {
        Ok(completion) => Json(completion).into_response(),
        Err(error) => shim_error(StatusCode::BAD_GATEWAY, &error.to_string()),
    }
}

async fn forward_responses_as_anthropic(
    client: &reqwest::Client,
    route: &NativeRoute,
    body: Value,
    lease: AdmissionLease,
) -> Response {
    let streaming = body.get("stream").and_then(Value::as_bool) == Some(true);
    let mut url = route.base_url().clone();
    let path = format!("{}/responses", url.path().trim_end_matches('/'));
    url.set_path(&path);
    let response = match client
        .post(url)
        .bearer_auth(route.api_key())
        .json(&body)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream request failed: {error}"),
            );
        }
    };
    let status = response.status();
    if streaming {
        if !status.is_success() {
            return forward_non_streaming_error(response).await;
        }
        return responses_sse_as_anthropic(response, lease);
    }
    let response = match response.json::<Value>().await {
        Ok(response) => response,
        Err(error) => {
            return shim_error(
                StatusCode::BAD_GATEWAY,
                &format!("upstream response was not JSON: {error}"),
            );
        }
    };
    if !status.is_success() {
        return (status, Json(response)).into_response();
    }
    match responses_to_anthropic_message_response(response) {
        Ok(message) => Json(message).into_response(),
        Err(error) => shim_error(StatusCode::BAD_GATEWAY, &error.to_string()),
    }
}

pub(crate) fn admission_error(error: AdmissionError) -> Response {
    match error {
        AdmissionError::QueueFull(_) => {
            shim_error(StatusCode::TOO_MANY_REQUESTS, &error.to_string())
        }
        AdmissionError::TimedOut(_) => {
            shim_error(StatusCode::SERVICE_UNAVAILABLE, &error.to_string())
        }
        AdmissionError::UnknownProvider(_) | AdmissionError::ClosedProvider(_) => {
            shim_error(StatusCode::BAD_GATEWAY, &error.to_string())
        }
        AdmissionError::ConflictingLimit { .. } => {
            shim_error(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string())
        }
    }
}

fn shim_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({ "error": { "message": message } })),
    )
        .into_response()
}
