use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::post;
use futures::StreamExt;
use pretty_assertions::assert_eq;
use serde_json::Value;
use tokio::sync::oneshot;

use super::NativeSettings;
use super::NativeShimServer;

type CapturedRequest = (Value, Option<String>, Option<String>, Option<String>);
type RequestCaptureSender = Arc<tokio::sync::Mutex<Option<oneshot::Sender<CapturedRequest>>>>;

#[test]
fn timed_out_admission_maps_to_service_unavailable() {
    let response =
        super::relay::admission_error(super::AdmissionError::TimedOut("primary".to_owned()));

    assert_eq!(
        response.status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn responses_relay_substitutes_the_upstream_model_and_uses_the_provider_key() {
    let (received_tx, received_rx) = oneshot::channel();
    let upstream = Router::new()
        .route("/v1/responses", post(capture_request))
        .with_state(Arc::new(tokio::sync::Mutex::new(Some(received_tx))));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"models":[{{"model":"upstream-sonnet","slug":"local-sonnet","provider":"openai-responses","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/responses"))
        .json(&serde_json::json!({"model":"local-sonnet", "input":"hello"}))
        .send()
        .await
        .expect("shim response");

    assert!(response.status().is_success());
    let received = received_rx.await.expect("upstream request");
    assert_eq!(received.0["model"], "upstream-sonnet");
    assert_eq!(received.1.as_deref(), Some("Bearer provider-secret"));
}

#[tokio::test]
async fn chat_completions_relay_substitutes_the_upstream_model_and_uses_the_provider_key() {
    let (received_tx, received_rx) = oneshot::channel();
    let upstream = Router::new()
        .route("/v1/chat/completions", post(capture_request))
        .with_state(Arc::new(tokio::sync::Mutex::new(Some(received_tx))));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"models":[{{"model":"upstream-gpt","slug":"local-gpt","provider":"openai-chat-completions","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model":"local-gpt",
            "messages":[{"role":"user","content":"hello"}]
        }))
        .send()
        .await
        .expect("shim response");

    assert!(response.status().is_success());
    let received = received_rx.await.expect("upstream request");
    assert_eq!(received.0["model"], "upstream-gpt");
    assert_eq!(received.1.as_deref(), Some("Bearer provider-secret"));
}

#[tokio::test]
async fn chat_completions_relay_translates_non_streaming_requests_for_anthropic_providers() {
    let upstream = Router::new().route("/v1/messages", post(anthropic_message_response));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"providers":[{{"id":"anthropic","protocol":"anthropic-messages","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}],"models":[{{"slug":"local-sonnet","upstream_model":"claude-sonnet","provider":"anthropic"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model":"local-sonnet",
            "messages":[{"role":"user","content":"hello"}],
            "max_tokens":64,
            "stream":false
        }))
        .send()
        .await
        .expect("shim response");

    assert!(response.status().is_success());
    let response: Value = response.json().await.expect("chat completion");
    assert_eq!(response["object"], "chat.completion");
    assert_eq!(
        response["choices"][0]["message"]["content"],
        "Hello from Claude"
    );
}

#[tokio::test]
async fn chat_completions_relay_translates_anthropic_sse_to_chat_chunks() {
    let upstream = Router::new().route("/v1/messages", post(anthropic_message_sse));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"providers":[{{"id":"anthropic","protocol":"anthropic-messages","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}],"models":[{{"slug":"local-sonnet","upstream_model":"claude-sonnet","provider":"anthropic"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model":"local-sonnet",
            "messages":[{"role":"user","content":"hello"}],
            "stream":true
        }))
        .send()
        .await
        .expect("shim response");

    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let body = response.text().await.expect("stream body");
    assert!(body.contains("\"object\":\"chat.completion.chunk\""));
    assert!(body.contains("\"content\":\"Hello\""));
    assert!(body.contains("data: [DONE]"));
}

#[tokio::test]
async fn chat_completions_relay_translates_non_streaming_requests_for_responses_providers() {
    let upstream = Router::new().route("/v1/responses", post(responses_payload));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"models":[{{"model":"upstream-gpt","slug":"local-gpt","provider":"openai-responses","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model":"local-gpt",
            "messages":[{"role":"user","content":"hello"}],
            "stream":false
        }))
        .send()
        .await
        .expect("shim response");

    assert!(response.status().is_success());
    let response: Value = response.json().await.expect("chat completion");
    assert_eq!(response["object"], "chat.completion");
    assert_eq!(
        response["choices"][0]["message"]["content"],
        "Hello from Responses"
    );
}

#[tokio::test]
async fn chat_completions_relay_translates_responses_sse_to_chat_chunks() {
    let upstream = Router::new().route("/v1/responses", post(responses_sse));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"models":[{{"model":"upstream-gpt","slug":"local-gpt","provider":"openai-responses","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model":"local-gpt",
            "messages":[{"role":"user","content":"hello"}],
            "stream":true
        }))
        .send()
        .await
        .expect("shim response");

    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let body = response.text().await.expect("stream body");
    assert!(body.contains("\"object\":\"chat.completion.chunk\""));
    assert!(body.contains("\"content\":\"Hello\""));
    assert!(body.contains("data: [DONE]"));
}

#[tokio::test]
async fn responses_relay_translates_non_streaming_requests_for_chat_completion_providers() {
    let upstream = Router::new().route("/v1/chat/completions", post(chat_completion_response));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"models":[{{"model":"upstream-gpt","slug":"local-gpt","provider":"openai-chat-completions","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/responses"))
        .json(&serde_json::json!({"model":"local-gpt", "input":"hello", "stream":false}))
        .send()
        .await
        .expect("shim response");

    assert!(response.status().is_success());
    let response: Value = response.json().await.expect("response payload");
    assert_eq!(response["object"], "response");
    assert_eq!(response["output"][0]["content"][0]["text"], "Hello back");
}

#[tokio::test]
async fn responses_relay_translates_chat_completion_sse_to_responses_sse() {
    let upstream = Router::new().route("/v1/chat/completions", post(chat_completion_sse));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"models":[{{"model":"upstream-gpt","slug":"local-gpt","provider":"openai-chat-completions","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/responses"))
        .json(&serde_json::json!({"model":"local-gpt", "input":"hello", "stream":true}))
        .send()
        .await
        .expect("shim response");

    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let body = response.text().await.expect("stream body");
    assert!(body.contains("event: response.created"));
    assert!(body.contains("event: response.output_text.delta"));
    assert!(body.contains("event: response.completed"));
}

#[tokio::test]
async fn responses_relay_translates_non_streaming_requests_for_anthropic_providers() {
    let upstream = Router::new().route("/v1/messages", post(anthropic_message_response));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"providers":[{{"id":"anthropic","protocol":"anthropic-messages","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}],"models":[{{"slug":"local-sonnet","upstream_model":"claude-sonnet","provider":"anthropic"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/responses"))
        .json(&serde_json::json!({
            "model":"local-sonnet",
            "input":"hello",
            "max_output_tokens":64,
            "stream":false
        }))
        .send()
        .await
        .expect("shim response");

    assert!(response.status().is_success());
    let response: Value = response.json().await.expect("response payload");
    assert_eq!(response["object"], "response");
    assert_eq!(
        response["output"][0]["content"][0]["text"],
        "Hello from Claude"
    );
}

#[tokio::test]
async fn responses_relay_translates_anthropic_sse_to_responses_sse() {
    let upstream = Router::new().route("/v1/messages", post(anthropic_message_sse));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"providers":[{{"id":"anthropic","protocol":"anthropic-messages","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}],"models":[{{"slug":"local-sonnet","upstream_model":"claude-sonnet","provider":"anthropic"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/responses"))
        .json(&serde_json::json!({
            "model":"local-sonnet",
            "input":"hello",
            "max_output_tokens":64,
            "stream":true
        }))
        .send()
        .await
        .expect("shim response");

    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let body = response.text().await.expect("stream body");
    assert!(body.contains("event: response.created"));
    assert!(body.contains("event: response.output_text.delta"));
    assert!(body.contains("event: response.completed"));
}

#[tokio::test]
async fn messages_relay_substitutes_the_upstream_model_and_sets_anthropic_version() {
    let (received_tx, received_rx) = oneshot::channel();
    let upstream = Router::new()
        .route("/v1/messages", post(capture_request))
        .with_state(Arc::new(tokio::sync::Mutex::new(Some(received_tx))));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"providers":[{{"id":"anthropic","protocol":"anthropic-messages","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}],"models":[{{"slug":"local-sonnet","upstream_model":"claude-sonnet","provider":"anthropic"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/messages"))
        .json(&serde_json::json!({
            "model":"local-sonnet",
            "max_tokens":64,
            "messages":[{"role":"user","content":"hello"}]
        }))
        .send()
        .await
        .expect("shim response");

    assert!(response.status().is_success());
    let received = received_rx.await.expect("upstream request");
    assert_eq!(received.0["model"], "claude-sonnet");
    assert_eq!(received.1.as_deref(), None);
    assert_eq!(received.2.as_deref(), Some("2023-06-01"));
    assert_eq!(received.3.as_deref(), Some("provider-secret"));
}

#[tokio::test]
async fn messages_relay_translates_non_streaming_requests_for_responses_providers() {
    let upstream = Router::new().route("/v1/responses", post(responses_payload));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"models":[{{"model":"upstream-gpt","slug":"local-gpt","provider":"openai-responses","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/messages"))
        .json(&serde_json::json!({
            "model":"local-gpt",
            "max_tokens":64,
            "messages":[{"role":"user","content":"hello"}],
            "stream":false
        }))
        .send()
        .await
        .expect("shim response");

    assert!(response.status().is_success());
    let response: Value = response.json().await.expect("message payload");
    assert_eq!(response["type"], "message");
    assert_eq!(response["content"][0]["text"], "Hello from Responses");
}

#[tokio::test]
async fn messages_relay_translates_responses_sse_to_anthropic_sse() {
    let upstream = Router::new().route("/v1/responses", post(responses_sse));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"models":[{{"model":"upstream-gpt","slug":"local-gpt","provider":"openai-responses","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/messages"))
        .json(&serde_json::json!({
            "model":"local-gpt",
            "max_tokens":64,
            "messages":[{"role":"user","content":"hello"}],
            "stream":true
        }))
        .send()
        .await
        .expect("shim response");

    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let body = response.text().await.expect("stream body");
    assert!(body.contains("event: message_start"));
    assert!(body.contains("event: content_block_delta"));
    assert!(body.contains("event: message_stop"));
}

#[tokio::test]
async fn messages_relay_translates_non_streaming_requests_for_chat_completion_providers() {
    let upstream = Router::new().route("/v1/chat/completions", post(chat_completion_response));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"models":[{{"model":"upstream-gpt","slug":"local-gpt","provider":"openai-chat-completions","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/messages"))
        .json(&serde_json::json!({
            "model":"local-gpt",
            "max_tokens":64,
            "messages":[{"role":"user","content":"hello"}],
            "stream":false
        }))
        .send()
        .await
        .expect("shim response");

    assert!(response.status().is_success());
    let response: Value = response.json().await.expect("message response");
    assert_eq!(response["type"], "message");
    assert_eq!(response["content"][0]["text"], "Hello back");
}

#[tokio::test]
async fn messages_relay_translates_chat_sse_to_anthropic_sse() {
    let upstream = Router::new().route("/v1/chat/completions", post(chat_completion_sse));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"models":[{{"model":"upstream-gpt","slug":"local-gpt","provider":"openai-chat-completions","base_url":"{upstream_url}/v1","api_key":"provider-secret"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/messages"))
        .json(&serde_json::json!({
            "model":"local-gpt",
            "max_tokens":64,
            "messages":[{"role":"user","content":"hello"}],
            "stream":true
        }))
        .send()
        .await
        .expect("shim response");

    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let body = response.text().await.expect("stream body");
    assert!(body.contains("event: message_start"));
    assert!(body.contains("event: content_block_delta"));
    assert!(body.contains("event: message_stop"));
}

#[tokio::test]
async fn dropping_a_client_stream_releases_the_provider_lease() {
    let upstream = Router::new().route("/v1/responses", post(never_ending_responses_sse));
    let upstream_url = start_server(upstream).await;
    let settings = NativeSettings::from_json(&format!(
        r#"{{"providers":[{{"id":"primary","protocol":"openai-responses","base_url":"{upstream_url}/v1","api_key":"provider-secret","concurrency":{{"max_active":1,"max_queued":1,"wait_timeout_ms":1000}}}}],"models":[{{"slug":"local","upstream_model":"upstream","provider":"primary"}}]}}"#
    ))
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let response = reqwest::Client::new()
        .post(format!("{shim_url}/v1/responses"))
        .json(&serde_json::json!({"model":"local", "input":"hello", "stream":true}))
        .send()
        .await
        .expect("shim response");
    assert!(response.status().is_success());

    let diagnostics = reqwest::get(format!("{shim_url}/diagnostics"))
        .await
        .expect("diagnostics")
        .text()
        .await
        .expect("diagnostics body");
    assert!(diagnostics.contains("\"active\":1"));

    drop(response);
    tokio::time::sleep(Duration::from_millis(50)).await;
    let diagnostics = reqwest::get(format!("{shim_url}/diagnostics"))
        .await
        .expect("diagnostics")
        .text()
        .await
        .expect("diagnostics body");
    assert!(diagnostics.contains("\"active\":0"));
}

#[tokio::test]
async fn diagnostics_exposes_capacity_but_not_provider_credentials() {
    let settings = NativeSettings::from_json(
        r#"{"providers":[{"id":"primary","protocol":"openai-responses","base_url":"https://provider.example/v1","api_key":"never-return-this","concurrency":{"max_active":1,"max_queued":2,"wait_timeout_ms":1000}}],"models":[{"slug":"local","upstream_model":"upstream","provider":"primary"}]}"#,
    )
    .expect("settings");
    let shim_url = start_server(
        NativeShimServer::new(Arc::new(settings))
            .expect("admission settings")
            .router(),
    )
    .await;

    let diagnostics = reqwest::get(format!("{shim_url}/diagnostics"))
        .await
        .expect("diagnostics response");
    assert!(diagnostics.status().is_success());
    let diagnostics = diagnostics.text().await.expect("diagnostics body");
    assert!(diagnostics.contains("primary"));
    assert!(diagnostics.contains("local"));
    assert!(!diagnostics.contains("never-return-this"));
    assert!(!diagnostics.contains("provider.example"));
}

#[tokio::test]
async fn runtime_binds_loopback_and_stops_on_shutdown() {
    let settings = NativeSettings::from_json(
        r#"{"models":[{"model":"upstream","slug":"local","provider":"openai-responses","base_url":"https://example.test/v1","api_key":"secret"}]}"#,
    )
    .expect("settings");
    let runtime = NativeShimServer::new(Arc::new(settings))
        .expect("admission settings")
        .start(None)
        .await
        .expect("loopback runtime");

    let health = reqwest::get(format!("{}/healthz", runtime.base_url()))
        .await
        .expect("health response");
    assert!(health.status().is_success());
    runtime.shutdown().await.expect("runtime shutdown");
}

async fn capture_request(
    State(sender): State<RequestCaptureSender>,
    headers: axum::http::HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    sender
        .lock()
        .await
        .take()
        .expect("single upstream request")
        .send((
            body,
            headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            headers
                .get("anthropic-version")
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            headers
                .get("x-api-key")
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
        ))
        .expect("receiver remains live");
    Json(serde_json::json!({"id":"response-1","object":"response"}))
}

async fn chat_completion_response() -> Json<Value> {
    Json(serde_json::json!({
        "id": "chatcmpl-1",
        "choices": [{
            "message": {"role": "assistant", "content": "Hello back"}
        }]
    }))
}

async fn chat_completion_sse() -> (
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
        "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\ndata: [DONE]\n\n",
    )
}

async fn anthropic_message_response() -> Json<Value> {
    Json(serde_json::json!({
        "id": "msg-1",
        "type": "message",
        "content": [{"type": "text", "text": "Hello from Claude"}],
        "stop_reason": "end_turn"
    }))
}

async fn anthropic_message_sse() -> (
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
        "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg-1\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
    )
}

async fn responses_payload() -> Json<Value> {
    Json(serde_json::json!({
        "id": "resp-1",
        "object": "response",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "Hello from Responses"}]
        }]
    }))
}

async fn responses_sse() -> (
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    (
        [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
        "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-1\"}}\n\nevent: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"Hello\"}\n\nevent: response.completed\ndata: {\"type\":\"response.completed\"}\n\n",
    )
}

async fn never_ending_responses_sse() -> Response {
    let first_event = futures::stream::once(async {
        Ok::<_, std::convert::Infallible>(axum::body::Bytes::from_static(
            b"event: response.created\ndata: {\"type\":\"response.created\"}\n\n",
        ))
    });
    let stream = first_event.chain(futures::stream::pending());
    (
        [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
        Body::from_stream(stream),
    )
        .into_response()
}

async fn start_server(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let address = listener.local_addr().expect("test server address");
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("test server should run");
    });
    format!("http://{address}")
}
