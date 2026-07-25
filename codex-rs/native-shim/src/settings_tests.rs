use std::time::Duration;

use pretty_assertions::assert_eq;

use super::NativeSettings;

#[test]
fn loads_a_valid_openai_compatible_route() {
    let settings = NativeSettings::from_json(
        r#"{
            "models": [{
                "model": "claude-sonnet",
                "slug": "claude-sonnet",
                "provider": "generic-chat-completion-api",
                "base_url": "https://api.example.test/v1/",
                "api_key": "secret",
                "provider_concurrency_limit": 1
            }]
        }"#,
    )
    .expect("valid settings");

    assert_eq!(settings.routes().len(), 1);
    assert_eq!(settings.routes()[0].model(), "claude-sonnet");
    assert_eq!(
        settings.routes()[0].provider_id(),
        "generic-chat-completion-api"
    );
    assert_eq!(
        settings.routes()[0].base_url().as_str(),
        "https://api.example.test/v1"
    );
    assert_eq!(settings.routes()[0].concurrency_limit(), Some(1));
    assert!(settings.routes()[0].has_api_key());
}

#[test]
fn loads_models_that_share_a_named_provider_entry() {
    let settings = NativeSettings::from_json(
        r#"{
            "providers": [{
                "id": "cheapinference",
                "protocol": "openai-responses",
                "base_url": "https://cheap.example.test/v1",
                "api_key": "secret",
                "concurrency": { "max_active": 1, "max_queued": 8, "wait_timeout_ms": 30000 }
            }],
            "models": [
                { "slug": "cheap-sonnet", "upstream_model": "sonnet", "provider": "cheapinference" },
                { "slug": "cheap-haiku", "upstream_model": "haiku", "provider": "cheapinference" }
            ]
        }"#,
    )
    .expect("preferred provider/model settings");

    assert_eq!(settings.routes().len(), 2);
    assert_eq!(settings.routes()[0].provider_id(), "cheapinference");
    assert_eq!(settings.routes()[1].provider_id(), "cheapinference");
    assert_eq!(settings.routes()[0].concurrency_limit(), Some(1));
    assert_eq!(settings.routes()[1].concurrency_limit(), Some(1));
    assert_eq!(
        settings.routes()[0].concurrency_policy(),
        Some(&super::ConcurrencyPolicy::new(
            1,
            8,
            Duration::from_secs(30),
        ))
    );
}

#[test]
fn loads_anthropic_provider_and_resolves_its_key_from_an_injected_environment() {
    let settings = NativeSettings::from_json_with_env(
        r#"{
            "providers": [{
                "id": "anthropic-main",
                "protocol": "anthropic-messages",
                "base_url": "https://api.anthropic.example/v1/",
                "api_key_env": "ANTHROPIC_API_KEY",
                "concurrency": { "max_active": 2, "max_queued": 3, "wait_timeout_ms": 125 }
            }],
            "models": [{ "slug": "sonnet", "upstream_model": "claude-sonnet", "provider": "anthropic-main" }]
        }"#,
        |name| (name == "ANTHROPIC_API_KEY").then_some("injected-secret".to_owned()),
    )
    .expect("valid Anthropic settings");

    let route = &settings.routes()[0];
    assert_eq!(route.protocol(), super::UpstreamProtocol::AnthropicMessages);
    assert_eq!(
        route.base_url().as_str(),
        "https://api.anthropic.example/v1"
    );
    assert!(route.has_api_key());
    assert_eq!(
        route.concurrency_policy(),
        Some(&super::ConcurrencyPolicy::new(
            2,
            3,
            Duration::from_millis(125),
        ))
    );
}

#[test]
fn legacy_routes_can_share_a_provider_entry_without_sharing_a_protocol_name() {
    let settings = NativeSettings::from_json(
        r#"{
            "models": [
                {
                    "model": "first",
                    "provider": "openai-responses",
                    "provider_entry_id": "shared-entry",
                    "base_url": "https://first.example/v1",
                    "api_key": "a",
                    "provider_concurrency_limit": 1
                },
                {
                    "model": "second",
                    "provider": "generic-chat-completion-api",
                    "provider_entry_id": "shared-entry",
                    "base_url": "https://second.example/v1",
                    "api_key": "b",
                    "provider_concurrency_limit": 1
                }
            ]
        }"#,
    )
    .expect("legacy provider entry settings");

    assert_eq!(settings.routes()[0].provider_id(), "shared-entry");
    assert_eq!(settings.routes()[1].provider_id(), "shared-entry");
    assert_eq!(
        settings.routes()[0].protocol(),
        super::UpstreamProtocol::OpenAiResponses
    );
    assert_eq!(
        settings.routes()[1].protocol(),
        super::UpstreamProtocol::OpenAiChatCompletions
    );
}

#[test]
fn rejects_zero_queue_capacity_in_a_provider_policy() {
    let error = NativeSettings::from_json(
        r#"{
            "providers": [{
                "id": "broken",
                "protocol": "openai-responses",
                "base_url": "https://api.example.test/v1",
                "api_key": "secret",
                "concurrency": { "max_active": 1, "max_queued": 0, "wait_timeout_ms": 30 }
            }],
            "models": [{ "slug": "broken", "upstream_model": "broken", "provider": "broken" }]
        }"#,
    )
    .expect_err("a bounded queue must have a positive capacity");

    assert_eq!(
        error.to_string(),
        "native shim queue capacity for provider `broken` must be positive"
    );
}

#[test]
fn rejects_duplicate_model_slugs() {
    let error = NativeSettings::from_json(
        r#"{
            "models": [
                {"model": "one", "slug": "shared", "provider": "openai", "base_url": "https://one.example/v1", "api_key": "a"},
                {"model": "two", "slug": "shared", "provider": "openai", "base_url": "https://two.example/v1", "api_key": "b"}
            ]
        }"#,
    )
    .expect_err("duplicate slugs must fail");

    assert_eq!(
        error.to_string(),
        "duplicate native shim model slug `shared`"
    );
}

#[test]
fn rejects_subscription_passthrough_routes() {
    let error = NativeSettings::from_json(
        r#"{
            "models": [{
                "model": "gpt-5.5",
                "provider": "chatgpt-passthrough",
                "base_url": "https://api.example.test/v1"
            }]
        }"#,
    )
    .expect_err("subscription routes are not native-v1 routes");

    assert_eq!(
        error.to_string(),
        "unsupported native shim provider `chatgpt-passthrough` for model `gpt-5.5`"
    );
}
