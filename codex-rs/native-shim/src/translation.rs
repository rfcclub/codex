use serde_json::Value;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("native shim only supports string Responses input for this provider adapter")]
    UnsupportedResponsesInput,
    #[error("native shim received a chat completion without a text assistant message")]
    UnsupportedChatCompletion,
}

pub fn responses_to_chat_completion(
    request: Value,
    upstream_model: &str,
) -> Result<Value, TranslationError> {
    let mut messages = Vec::new();
    if let Some(instructions) = request.get("instructions").and_then(Value::as_str) {
        messages.push(json!({"role": "system", "content": instructions}));
    }
    messages.extend(responses_input_to_chat_messages(
        request
            .get("input")
            .ok_or(TranslationError::UnsupportedResponsesInput)?,
    )?);
    let mut translated = serde_json::Map::new();
    translated.insert("model".to_owned(), Value::String(upstream_model.to_owned()));
    translated.insert("messages".to_owned(), Value::Array(messages));
    if let Some(stream) = request.get("stream") {
        translated.insert("stream".to_owned(), stream.clone());
    }
    Ok(Value::Object(translated))
}

fn responses_input_to_chat_messages(input: &Value) -> Result<Vec<Value>, TranslationError> {
    if let Some(input) = input.as_str() {
        return Ok(vec![json!({"role": "user", "content": input})]);
    }
    let input = input
        .as_array()
        .filter(|input| !input.is_empty())
        .ok_or(TranslationError::UnsupportedResponsesInput)?;
    input
        .iter()
        .map(|item| {
            let role = item.get("role").and_then(Value::as_str).unwrap_or("user");
            let content = match item.get("content") {
                Some(Value::String(content)) => content.clone(),
                Some(Value::Array(content)) => content
                    .iter()
                    .filter(|part| {
                        matches!(
                            part.get("type").and_then(Value::as_str),
                            Some("input_text" | "output_text")
                        )
                    })
                    .filter_map(|part| part.get("text").and_then(Value::as_str))
                    .collect::<String>(),
                _ => String::new(),
            };
            if content.is_empty() {
                return Err(TranslationError::UnsupportedResponsesInput);
            }
            let role = match role {
                "developer" | "system" => "system",
                "assistant" => "assistant",
                _ => "user",
            };
            Ok(json!({"role": role, "content": content}))
        })
        .collect()
}

pub fn chat_completion_to_response(completion: Value) -> Result<Value, TranslationError> {
    let id = completion
        .get("id")
        .and_then(Value::as_str)
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    let text = completion
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    Ok(text_response(id, text))
}

pub fn chat_completion_to_anthropic_message(
    request: Value,
    upstream_model: &str,
) -> Result<Value, TranslationError> {
    let messages = request
        .get("messages")
        .and_then(Value::as_array)
        .filter(|messages| !messages.is_empty())
        .ok_or(TranslationError::UnsupportedResponsesInput)?;
    let system = messages
        .iter()
        .filter(|message| message.get("role").and_then(Value::as_str) == Some("system"))
        .filter_map(|message| message.get("content").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let anthropic_messages = messages
        .iter()
        .filter(|message| message.get("role").and_then(Value::as_str) != Some("system"))
        .cloned()
        .collect::<Vec<_>>();
    if anthropic_messages.is_empty() {
        return Err(TranslationError::UnsupportedResponsesInput);
    }
    let mut translated = serde_json::Map::new();
    translated.insert("model".to_owned(), Value::String(upstream_model.to_owned()));
    translated.insert("messages".to_owned(), Value::Array(anthropic_messages));
    translated.insert(
        "max_tokens".to_owned(),
        request
            .get("max_tokens")
            .or_else(|| request.get("max_completion_tokens"))
            .cloned()
            .unwrap_or_else(|| json!(1024)),
    );
    if !system.is_empty() {
        translated.insert("system".to_owned(), Value::String(system));
    }
    if let Some(stream) = request.get("stream") {
        translated.insert("stream".to_owned(), stream.clone());
    }
    Ok(Value::Object(translated))
}

pub fn anthropic_message_to_chat_completion(
    message: Value,
    model: &str,
) -> Result<Value, TranslationError> {
    let id = message
        .get("id")
        .and_then(Value::as_str)
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    let text = message
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| {
            content.iter().find_map(|item| {
                (item.get("type").and_then(Value::as_str) == Some("text"))
                    .then(|| item.get("text").and_then(Value::as_str))
                    .flatten()
            })
        })
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    Ok(json!({
        "id": id,
        "object": "chat.completion",
        "created": 0,
        "model": model,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": text},
            "finish_reason": "stop"
        }]
    }))
}

pub fn chat_completion_to_responses_request(
    request: Value,
    upstream_model: &str,
) -> Result<Value, TranslationError> {
    let messages = request
        .get("messages")
        .and_then(Value::as_array)
        .filter(|messages| !messages.is_empty())
        .ok_or(TranslationError::UnsupportedResponsesInput)?;
    let instructions = messages
        .iter()
        .filter(|message| message.get("role").and_then(Value::as_str) == Some("system"))
        .filter_map(|message| message.get("content").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let input = messages
        .iter()
        .filter(|message| message.get("role").and_then(Value::as_str) != Some("system"))
        .cloned()
        .collect::<Vec<_>>();
    if input.is_empty() {
        return Err(TranslationError::UnsupportedResponsesInput);
    }
    let mut translated = serde_json::Map::new();
    translated.insert("model".to_owned(), Value::String(upstream_model.to_owned()));
    translated.insert("input".to_owned(), Value::Array(input));
    if !instructions.is_empty() {
        translated.insert("instructions".to_owned(), Value::String(instructions));
    }
    if let Some(max_output_tokens) = request
        .get("max_tokens")
        .or_else(|| request.get("max_completion_tokens"))
    {
        translated.insert("max_output_tokens".to_owned(), max_output_tokens.clone());
    }
    if let Some(stream) = request.get("stream") {
        translated.insert("stream".to_owned(), stream.clone());
    }
    Ok(Value::Object(translated))
}

pub fn response_to_chat_completion(
    response: Value,
    model: &str,
) -> Result<Value, TranslationError> {
    let id = response
        .get("id")
        .and_then(Value::as_str)
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    let text = response
        .get("output")
        .and_then(Value::as_array)
        .and_then(|output| {
            output
                .iter()
                .find(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        })
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array)
        .and_then(|content| {
            content.iter().find_map(|item| {
                (item.get("type").and_then(Value::as_str) == Some("output_text"))
                    .then(|| item.get("text").and_then(Value::as_str))
                    .flatten()
            })
        })
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    Ok(json!({
        "id": id,
        "object": "chat.completion",
        "created": 0,
        "model": model,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": text},
            "finish_reason": "stop"
        }]
    }))
}

pub fn anthropic_message_to_chat_completion_request(
    request: Value,
    upstream_model: &str,
) -> Result<Value, TranslationError> {
    let messages = request
        .get("messages")
        .and_then(Value::as_array)
        .filter(|messages| !messages.is_empty())
        .ok_or(TranslationError::UnsupportedResponsesInput)?;
    let mut chat_messages = Vec::new();
    if let Some(system) = request.get("system").and_then(Value::as_str) {
        chat_messages.push(json!({"role": "system", "content": system}));
    }
    chat_messages.extend(messages.iter().cloned());
    let mut translated = serde_json::Map::new();
    translated.insert("model".to_owned(), Value::String(upstream_model.to_owned()));
    translated.insert("messages".to_owned(), Value::Array(chat_messages));
    if let Some(max_tokens) = request.get("max_tokens") {
        translated.insert("max_tokens".to_owned(), max_tokens.clone());
    }
    if let Some(stream) = request.get("stream") {
        translated.insert("stream".to_owned(), stream.clone());
    }
    Ok(Value::Object(translated))
}

pub fn chat_completion_to_anthropic_message_response(
    completion: Value,
) -> Result<Value, TranslationError> {
    let id = completion
        .get("id")
        .and_then(Value::as_str)
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    let text = completion
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    Ok(json!({
        "id": id,
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": text}],
        "stop_reason": "end_turn"
    }))
}

pub fn responses_to_anthropic_message(
    request: Value,
    upstream_model: &str,
) -> Result<Value, TranslationError> {
    let input = request
        .get("input")
        .and_then(Value::as_str)
        .ok_or(TranslationError::UnsupportedResponsesInput)?;
    let mut translated = serde_json::Map::new();
    translated.insert("model".to_owned(), Value::String(upstream_model.to_owned()));
    translated.insert(
        "max_tokens".to_owned(),
        request
            .get("max_output_tokens")
            .cloned()
            .unwrap_or_else(|| json!(1024)),
    );
    translated.insert(
        "messages".to_owned(),
        json!([{"role": "user", "content": input}]),
    );
    if let Some(instructions) = request.get("instructions").and_then(Value::as_str) {
        translated.insert("system".to_owned(), Value::String(instructions.to_owned()));
    }
    if let Some(stream) = request.get("stream") {
        translated.insert("stream".to_owned(), stream.clone());
    }
    Ok(Value::Object(translated))
}

pub fn anthropic_message_to_response(message: Value) -> Result<Value, TranslationError> {
    let id = message
        .get("id")
        .and_then(Value::as_str)
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    let text = message
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| {
            content.iter().find_map(|item| {
                (item.get("type").and_then(Value::as_str) == Some("text"))
                    .then(|| item.get("text").and_then(Value::as_str))
                    .flatten()
            })
        })
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    Ok(text_response(id, text))
}

pub fn anthropic_message_to_responses_request(
    request: Value,
    upstream_model: &str,
) -> Result<Value, TranslationError> {
    let messages = request
        .get("messages")
        .and_then(Value::as_array)
        .filter(|messages| !messages.is_empty())
        .ok_or(TranslationError::UnsupportedResponsesInput)?;
    let mut translated = serde_json::Map::new();
    translated.insert("model".to_owned(), Value::String(upstream_model.to_owned()));
    translated.insert("input".to_owned(), Value::Array(messages.clone()));
    translated.insert(
        "max_output_tokens".to_owned(),
        request
            .get("max_tokens")
            .cloned()
            .unwrap_or_else(|| json!(1024)),
    );
    if let Some(system) = request.get("system").and_then(Value::as_str) {
        translated.insert("instructions".to_owned(), Value::String(system.to_owned()));
    }
    if let Some(stream) = request.get("stream") {
        translated.insert("stream".to_owned(), stream.clone());
    }
    Ok(Value::Object(translated))
}

pub fn responses_to_anthropic_message_response(response: Value) -> Result<Value, TranslationError> {
    let id = response
        .get("id")
        .and_then(Value::as_str)
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    let text = response
        .get("output")
        .and_then(Value::as_array)
        .and_then(|output| {
            output
                .iter()
                .find(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        })
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array)
        .and_then(|content| {
            content.iter().find_map(|item| {
                (item.get("type").and_then(Value::as_str) == Some("output_text"))
                    .then(|| item.get("text").and_then(Value::as_str))
                    .flatten()
            })
        })
        .ok_or(TranslationError::UnsupportedChatCompletion)?;
    Ok(json!({
        "id": id,
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": text}],
        "stop_reason": "end_turn"
    }))
}

fn text_response(id: &str, text: &str) -> Value {
    json!({
        "id": id,
        "object": "response",
        "status": "completed",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": text}]
        }]
    })
}

/// Incrementally converts OpenAI Chat Completions SSE frames into the subset
/// of Responses SSE events consumed by Codex. The internal buffer makes the
/// conversion independent of HTTP chunk boundaries.
#[derive(Default)]
pub struct ChatSseToResponses {
    buffer: Vec<u8>,
    response_id: Option<String>,
    output_text: String,
}

/// Incrementally converts Anthropic Messages SSE frames into the subset of
/// Responses SSE events consumed by Codex.
#[derive(Default)]
pub struct AnthropicSseToResponses {
    buffer: Vec<u8>,
    response_id: Option<String>,
}

/// Incrementally converts OpenAI Responses SSE frames into Anthropic Messages
/// events for clients that use the Messages ingress.
#[derive(Default)]
pub struct ResponsesSseToAnthropic {
    buffer: Vec<u8>,
    started: bool,
}

/// Incrementally converts Anthropic Messages SSE frames into OpenAI Chat
/// Completions chunks.
#[derive(Default)]
pub struct AnthropicSseToChat {
    buffer: Vec<u8>,
    completion_id: Option<String>,
}

/// Incrementally converts OpenAI Responses SSE frames into OpenAI Chat
/// Completions chunks.
#[derive(Default)]
pub struct ResponsesSseToChat {
    buffer: Vec<u8>,
    completion_id: Option<String>,
}

/// Incrementally converts OpenAI Chat Completions SSE frames into Anthropic
/// Messages events.
#[derive(Default)]
pub struct ChatSseToAnthropic {
    buffer: Vec<u8>,
    started: bool,
}

impl ChatSseToResponses {
    /// Accept raw HTTP body bytes without assuming UTF-8 boundaries line up
    /// with HTTP chunk boundaries.
    pub fn push_bytes(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);
        let mut output = Vec::new();
        while let Some(frame_end) = self.buffer.windows(2).position(|window| window == b"\n\n") {
            let frame = self.buffer.drain(..frame_end + 2).collect::<Vec<_>>();
            let Ok(frame) = std::str::from_utf8(&frame) else {
                continue;
            };
            let data = frame
                .lines()
                .filter_map(|line| line.strip_prefix("data: "))
                .collect::<Vec<_>>()
                .join("\n");
            if data == "[DONE]" {
                if let Some(response_id) = self.response_id.as_deref() {
                    output.push(output_item_done_event(response_id, &self.output_text));
                    output.push(completed_event(response_id));
                }
                continue;
            }
            let Ok(chunk) = serde_json::from_str::<Value>(&data) else {
                continue;
            };
            if self.response_id.is_none()
                && let Some(response_id) = chunk.get("id").and_then(Value::as_str)
            {
                self.response_id = Some(response_id.to_owned());
                output.push(created_event(response_id));
                output.push(output_item_added_event(response_id));
            }
            if let Some(delta) = chunk
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first())
                .and_then(|choice| choice.get("delta"))
                .and_then(|delta| delta.get("content"))
                .and_then(Value::as_str)
            {
                self.output_text.push_str(delta);
                output.push(text_delta_event(delta));
            }
        }
        output
    }
}

impl AnthropicSseToResponses {
    /// Accept raw HTTP body bytes without assuming UTF-8 boundaries line up
    /// with HTTP chunk boundaries.
    pub fn push_bytes(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);
        let mut output = Vec::new();
        while let Some(frame_end) = self.buffer.windows(2).position(|window| window == b"\n\n") {
            let frame = self.buffer.drain(..frame_end + 2).collect::<Vec<_>>();
            let Ok(frame) = std::str::from_utf8(&frame) else {
                continue;
            };
            let event = frame.lines().find_map(|line| line.strip_prefix("event: "));
            let data = frame
                .lines()
                .filter_map(|line| line.strip_prefix("data: "))
                .collect::<Vec<_>>()
                .join("\n");
            let Ok(payload) = serde_json::from_str::<Value>(&data) else {
                continue;
            };
            match event {
                Some("message_start") => {
                    let Some(response_id) = payload
                        .get("message")
                        .and_then(|message| message.get("id"))
                        .and_then(Value::as_str)
                    else {
                        continue;
                    };
                    self.response_id = Some(response_id.to_owned());
                    output.push(created_event(response_id));
                }
                Some("content_block_delta") => {
                    if let Some(delta) = payload
                        .get("delta")
                        .and_then(|delta| delta.get("text"))
                        .and_then(Value::as_str)
                    {
                        output.push(text_delta_event(delta));
                    }
                }
                Some("message_stop") => {
                    if let Some(response_id) = self.response_id.as_deref() {
                        output.push(completed_event(response_id));
                    }
                }
                _ => {}
            }
        }
        output
    }
}

impl ResponsesSseToAnthropic {
    /// Accept raw HTTP body bytes without assuming UTF-8 boundaries line up
    /// with HTTP chunk boundaries.
    pub fn push_bytes(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);
        let mut output = Vec::new();
        while let Some(frame_end) = self.buffer.windows(2).position(|window| window == b"\n\n") {
            let frame = self.buffer.drain(..frame_end + 2).collect::<Vec<_>>();
            let Ok(frame) = std::str::from_utf8(&frame) else {
                continue;
            };
            let event = frame.lines().find_map(|line| line.strip_prefix("event: "));
            let data = frame
                .lines()
                .filter_map(|line| line.strip_prefix("data: "))
                .collect::<Vec<_>>()
                .join("\n");
            let Ok(payload) = serde_json::from_str::<Value>(&data) else {
                continue;
            };
            match event {
                Some("response.created") => {
                    let Some(response_id) = payload
                        .get("response")
                        .and_then(|response| response.get("id"))
                        .and_then(Value::as_str)
                    else {
                        continue;
                    };
                    if !self.started {
                        self.started = true;
                        output.push(anthropic_message_start_event(response_id));
                        output.push(anthropic_content_block_start_event());
                    }
                }
                Some("response.output_text.delta") => {
                    if let Some(delta) = payload.get("delta").and_then(Value::as_str) {
                        output.push(anthropic_text_delta_event(delta));
                    }
                }
                Some("response.completed") if self.started => {
                    output.push(anthropic_content_block_stop_event());
                    output.push(anthropic_message_delta_event());
                    output.push(anthropic_message_stop_event());
                }
                _ => {}
            }
        }
        output
    }
}

impl AnthropicSseToChat {
    /// Accept raw HTTP body bytes without assuming UTF-8 boundaries line up
    /// with HTTP chunk boundaries.
    pub fn push_bytes(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);
        let mut output = Vec::new();
        while let Some(frame_end) = self.buffer.windows(2).position(|window| window == b"\n\n") {
            let frame = self.buffer.drain(..frame_end + 2).collect::<Vec<_>>();
            let Ok(frame) = std::str::from_utf8(&frame) else {
                continue;
            };
            let event = frame.lines().find_map(|line| line.strip_prefix("event: "));
            let data = frame
                .lines()
                .filter_map(|line| line.strip_prefix("data: "))
                .collect::<Vec<_>>()
                .join("\n");
            let Ok(payload) = serde_json::from_str::<Value>(&data) else {
                continue;
            };
            match event {
                Some("message_start") => {
                    let Some(id) = payload
                        .get("message")
                        .and_then(|message| message.get("id"))
                        .and_then(Value::as_str)
                    else {
                        continue;
                    };
                    self.completion_id = Some(id.to_owned());
                    output.push(chat_chunk(id, json!({"role": "assistant"}), None));
                }
                Some("content_block_delta") => {
                    if let (Some(id), Some(text)) = (
                        self.completion_id.as_deref(),
                        payload
                            .get("delta")
                            .and_then(|delta| delta.get("text"))
                            .and_then(Value::as_str),
                    ) {
                        output.push(chat_chunk(id, json!({"content": text}), None));
                    }
                }
                Some("message_stop") => {
                    if let Some(id) = self.completion_id.as_deref() {
                        output.push(format!(
                            "{}data: [DONE]\n\n",
                            chat_chunk(id, json!({}), Some("stop"))
                        ));
                    }
                }
                _ => {}
            }
        }
        output
    }
}

impl ResponsesSseToChat {
    /// Accept raw HTTP body bytes without assuming UTF-8 boundaries line up
    /// with HTTP chunk boundaries.
    pub fn push_bytes(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);
        let mut output = Vec::new();
        while let Some(frame_end) = self.buffer.windows(2).position(|window| window == b"\n\n") {
            let frame = self.buffer.drain(..frame_end + 2).collect::<Vec<_>>();
            let Ok(frame) = std::str::from_utf8(&frame) else {
                continue;
            };
            let event = frame.lines().find_map(|line| line.strip_prefix("event: "));
            let data = frame
                .lines()
                .filter_map(|line| line.strip_prefix("data: "))
                .collect::<Vec<_>>()
                .join("\n");
            let Ok(payload) = serde_json::from_str::<Value>(&data) else {
                continue;
            };
            match event {
                Some("response.created") => {
                    let Some(id) = payload
                        .get("response")
                        .and_then(|response| response.get("id"))
                        .and_then(Value::as_str)
                    else {
                        continue;
                    };
                    self.completion_id = Some(id.to_owned());
                    output.push(chat_chunk(id, json!({"role": "assistant"}), None));
                }
                Some("response.output_text.delta") => {
                    if let (Some(id), Some(text)) = (
                        self.completion_id.as_deref(),
                        payload.get("delta").and_then(Value::as_str),
                    ) {
                        output.push(chat_chunk(id, json!({"content": text}), None));
                    }
                }
                Some("response.completed") => {
                    if let Some(id) = self.completion_id.as_deref() {
                        output.push(format!(
                            "{}data: [DONE]\n\n",
                            chat_chunk(id, json!({}), Some("stop"))
                        ));
                    }
                }
                _ => {}
            }
        }
        output
    }
}

impl ChatSseToAnthropic {
    /// Accept raw HTTP body bytes without assuming UTF-8 boundaries line up
    /// with HTTP chunk boundaries.
    pub fn push_bytes(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);
        let mut output = Vec::new();
        while let Some(frame_end) = self.buffer.windows(2).position(|window| window == b"\n\n") {
            let frame = self.buffer.drain(..frame_end + 2).collect::<Vec<_>>();
            let Ok(frame) = std::str::from_utf8(&frame) else {
                continue;
            };
            let data = frame
                .lines()
                .filter_map(|line| line.strip_prefix("data: "))
                .collect::<Vec<_>>()
                .join("\n");
            if data == "[DONE]" {
                if self.started {
                    output.push(anthropic_content_block_stop_event());
                    output.push(anthropic_message_delta_event());
                    output.push(anthropic_message_stop_event());
                }
                continue;
            }
            let Ok(payload) = serde_json::from_str::<Value>(&data) else {
                continue;
            };
            if !self.started
                && let Some(id) = payload.get("id").and_then(Value::as_str)
            {
                self.started = true;
                output.push(anthropic_message_start_event(id));
                output.push(anthropic_content_block_start_event());
            }
            if let Some(text) = payload
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first())
                .and_then(|choice| choice.get("delta"))
                .and_then(|delta| delta.get("content"))
                .and_then(Value::as_str)
            {
                output.push(anthropic_text_delta_event(text));
            }
        }
        output
    }
}

fn created_event(response_id: &str) -> String {
    let response_id = Value::String(response_id.to_owned()).to_string();
    format!(
        "event: response.created\ndata: {{\"type\":\"response.created\",\"response\":{{\"id\":{response_id},\"object\":\"response\",\"status\":\"in_progress\"}}}}\n\n"
    )
}

fn text_delta_event(delta: &str) -> String {
    let delta = Value::String(delta.to_owned()).to_string();
    format!(
        "event: response.output_text.delta\ndata: {{\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":{delta}}}\n\n"
    )
}

fn output_item_added_event(response_id: &str) -> String {
    let response_id = Value::String(response_id.to_owned()).to_string();
    format!(
        "event: response.output_item.added\ndata: {{\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{{\"type\":\"message\",\"role\":\"assistant\",\"id\":{response_id},\"content\":[{{\"type\":\"output_text\",\"text\":\"\"}}]}}}}\n\n"
    )
}

fn output_item_done_event(response_id: &str, text: &str) -> String {
    let response_id = Value::String(response_id.to_owned()).to_string();
    let text = Value::String(text.to_owned()).to_string();
    format!(
        "event: response.output_item.done\ndata: {{\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{{\"type\":\"message\",\"role\":\"assistant\",\"id\":{response_id},\"content\":[{{\"type\":\"output_text\",\"text\":{text}}}]}}}}\n\n"
    )
}

fn completed_event(response_id: &str) -> String {
    let response_id = Value::String(response_id.to_owned()).to_string();
    format!(
        "event: response.completed\ndata: {{\"type\":\"response.completed\",\"response\":{{\"id\":{response_id},\"object\":\"response\",\"status\":\"completed\"}}}}\n\n"
    )
}

fn anthropic_sse_event(event: &str, payload: Value) -> String {
    format!("event: {event}\ndata: {payload}\n\n")
}

fn anthropic_message_start_event(response_id: &str) -> String {
    anthropic_sse_event(
        "message_start",
        json!({
            "type": "message_start",
            "message": {
                "id": response_id,
                "type": "message",
                "role": "assistant",
                "content": []
            }
        }),
    )
}

fn anthropic_content_block_start_event() -> String {
    anthropic_sse_event(
        "content_block_start",
        json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""}
        }),
    )
}

fn anthropic_text_delta_event(delta: &str) -> String {
    anthropic_sse_event(
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": delta}
        }),
    )
}

fn anthropic_content_block_stop_event() -> String {
    anthropic_sse_event(
        "content_block_stop",
        json!({"type": "content_block_stop", "index": 0}),
    )
}

fn anthropic_message_delta_event() -> String {
    anthropic_sse_event(
        "message_delta",
        json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn"},
            "usage": {"output_tokens": 0}
        }),
    )
}

fn anthropic_message_stop_event() -> String {
    anthropic_sse_event("message_stop", json!({"type": "message_stop"}))
}

fn chat_chunk(id: &str, delta: Value, finish_reason: Option<&str>) -> String {
    let mut choice = serde_json::Map::new();
    choice.insert("index".to_owned(), json!(0));
    choice.insert("delta".to_owned(), delta);
    if let Some(finish_reason) = finish_reason {
        choice.insert("finish_reason".to_owned(), json!(finish_reason));
    }
    format!(
        "data: {}\n\n",
        json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": 0,
            "choices": [choice]
        })
    )
}
