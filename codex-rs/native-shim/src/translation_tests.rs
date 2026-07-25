use pretty_assertions::assert_eq;
use serde_json::json;

use super::translation::AnthropicSseToChat;
use super::translation::AnthropicSseToResponses;
use super::translation::ChatSseToAnthropic;
use super::translation::ChatSseToResponses;
use super::translation::ResponsesSseToAnthropic;
use super::translation::ResponsesSseToChat;
use super::translation::anthropic_message_to_response;
use super::translation::anthropic_message_to_responses_request;
use super::translation::chat_completion_to_response;
use super::translation::responses_to_anthropic_message;
use super::translation::responses_to_anthropic_message_response;
use super::translation::responses_to_chat_completion;

#[test]
fn translates_a_text_responses_request_to_chat_completions() {
    let request = responses_to_chat_completion(
        json!({
            "model": "local-model",
            "instructions": "Be concise.",
            "input": "Hello",
            "stream": false
        }),
        "upstream-model",
    )
    .expect("supported text request");

    assert_eq!(
        request,
        json!({
            "model": "upstream-model",
            "stream": false,
            "messages": [
                {"role": "system", "content": "Be concise."},
                {"role": "user", "content": "Hello"}
            ]
        })
    );
}

#[test]
fn translates_a_non_streaming_chat_completion_to_a_responses_payload() {
    let response = chat_completion_to_response(json!({
        "id": "chatcmpl-1",
        "model": "upstream-model",
        "choices": [{
            "index": 0,
            "finish_reason": "stop",
            "message": {"role": "assistant", "content": "Hello back"}
        }]
    }))
    .expect("supported completion");

    assert_eq!(
        response,
        json!({
            "id": "chatcmpl-1",
            "object": "response",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Hello back"}]
            }]
        })
    );
}

#[test]
fn translates_a_text_responses_request_to_anthropic_messages() {
    let request = responses_to_anthropic_message(
        json!({
            "model": "local-model",
            "instructions": "Be concise.",
            "input": "Hello",
            "max_output_tokens": 77,
            "stream": false
        }),
        "claude-sonnet",
    )
    .expect("supported text request");

    assert_eq!(
        request,
        json!({
            "model": "claude-sonnet",
            "system": "Be concise.",
            "max_tokens": 77,
            "stream": false,
            "messages": [{"role": "user", "content": "Hello"}]
        })
    );
}

#[test]
fn translates_anthropic_text_message_to_a_responses_payload() {
    let response = anthropic_message_to_response(json!({
        "id": "msg-1",
        "type": "message",
        "content": [{"type": "text", "text": "Hello back"}],
        "stop_reason": "end_turn"
    }))
    .expect("supported message");

    assert_eq!(
        response,
        json!({
            "id": "msg-1",
            "object": "response",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Hello back"}]
            }]
        })
    );
}

#[test]
fn translates_anthropic_messages_request_to_responses() {
    let request = anthropic_message_to_responses_request(
        json!({
            "model": "local-model",
            "system": "Be concise.",
            "max_tokens": 77,
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": false
        }),
        "upstream-model",
    )
    .expect("supported text request");

    assert_eq!(
        request,
        json!({
            "model": "upstream-model",
            "instructions": "Be concise.",
            "max_output_tokens": 77,
            "input": [{"role": "user", "content": "Hello"}],
            "stream": false
        })
    );
}

#[test]
fn translates_a_responses_text_payload_to_anthropic_message() {
    let response = responses_to_anthropic_message_response(json!({
        "id": "resp-1",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "Hello back"}]
        }]
    }))
    .expect("supported response");

    assert_eq!(
        response,
        json!({
            "id": "resp-1",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello back"}],
            "stop_reason": "end_turn"
        })
    );
}

#[test]
fn translates_chat_sse_frames_to_responses_sse_events_across_split_chunks() {
    let mut adapter = ChatSseToResponses::default();
    let fixture = include_str!("../tests/fixtures/chat-to-responses.input.sse");
    let split_at = fixture.find("Hello").expect("fixture text") + 3;
    let first = adapter.push_bytes(&fixture.as_bytes()[..split_at]);
    assert!(first.is_empty());
    let second = adapter.push_bytes(&fixture.as_bytes()[split_at..]);

    assert_eq!(
        second.concat(),
        include_str!("../tests/fixtures/chat-to-responses.output.sse")
    );
}

#[test]
fn translates_chat_sse_when_a_utf8_character_is_split_between_http_chunks() {
    let mut adapter = ChatSseToResponses::default();
    let frame = "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"delta\":{\"content\":\"hé\"}}]}\n\n";
    let split_at = frame.find("é").expect("accented character") + 1;

    assert!(adapter.push_bytes(&frame.as_bytes()[..split_at]).is_empty());
    let events = adapter.push_bytes(&frame.as_bytes()[split_at..]);

    assert!(events.iter().any(|event| event.contains("\"hé\"")));
}

#[test]
fn translates_anthropic_sse_frames_to_responses_events() {
    let mut adapter = AnthropicSseToResponses::default();
    let events = adapter.push_bytes(include_bytes!(
        "../tests/fixtures/anthropic-to-responses.input.sse"
    ));

    assert_eq!(
        events.concat(),
        include_str!("../tests/fixtures/anthropic-to-responses.output.sse")
    );
}

#[test]
fn translates_responses_sse_frames_to_anthropic_events() {
    let mut adapter = ResponsesSseToAnthropic::default();
    let events = adapter.push_bytes(include_bytes!(
        "../tests/fixtures/responses-to-anthropic.input.sse"
    ));

    assert_eq!(
        events.concat(),
        include_str!("../tests/fixtures/responses-to-anthropic.output.sse")
    );
}

#[test]
fn translates_anthropic_sse_frames_to_chat_completion_chunks() {
    let mut adapter = AnthropicSseToChat::default();
    let events = adapter.push_bytes(include_bytes!(
        "../tests/fixtures/anthropic-to-chat.input.sse"
    ));

    assert_eq!(
        events.concat(),
        include_str!("../tests/fixtures/anthropic-to-chat.output.sse")
    );
}

#[test]
fn translates_responses_sse_frames_to_chat_completion_chunks() {
    let mut adapter = ResponsesSseToChat::default();
    let events = adapter.push_bytes(include_bytes!(
        "../tests/fixtures/responses-to-chat.input.sse"
    ));

    assert_eq!(
        events.concat(),
        include_str!("../tests/fixtures/responses-to-chat.output.sse")
    );
}

#[test]
fn translates_chat_sse_frames_to_anthropic_events() {
    let mut adapter = ChatSseToAnthropic::default();
    let events = adapter.push_bytes(include_bytes!(
        "../tests/fixtures/chat-to-anthropic.input.sse"
    ));

    assert_eq!(
        events.concat(),
        include_str!("../tests/fixtures/chat-to-anthropic.output.sse")
    );
}
