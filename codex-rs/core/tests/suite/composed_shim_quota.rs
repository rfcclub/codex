use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_model_provider_info::ModelProviderInfo;
use codex_protocol::protocol::CodexErrorInfo;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::ThreadSettingsOverrides;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::json;
use test_case::test_case;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

const CUSTOM_MODEL: &str = "custom/quota-isolated";

#[derive(Clone, Copy)]
enum Compaction {
    Automatic,
    Manual,
}

#[test_case(Compaction::Automatic; "automatic compaction")]
#[test_case(Compaction::Manual; "manual compaction")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn custom_compaction_does_not_consult_exhausted_native_provider(
    compaction: Compaction,
) -> Result<()> {
    skip_if_no_network!(Ok(()));
    let native = MockServer::start().await;
    let custom = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "error": {"type": "usage_limit_reached", "message": "native quota exhausted"}
        })))
        .mount(&native)
        .await;
    let response_count = match compaction {
        Compaction::Automatic => 3,
        Compaction::Manual => 2,
    };
    let custom_requests = responses::mount_sse_sequence(
        &custom,
        (0..response_count)
            .map(|index| {
                let id = format!("custom-response-{index}");
                responses::sse(vec![
                    responses::ev_assistant_message(&id, "custom provider reply"),
                    responses::ev_completed_with_tokens(
                        &id,
                        /*total_tokens*/ if index == 0 { 10 } else { 0 },
                    ),
                ])
            })
            .collect(),
    )
    .await;
    let custom_url = format!("{}/v1", custom.uri());
    let mut builder = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_model_info_override(CUSTOM_MODEL, move |model| {
            model.tool_mode = None;
            model.multi_agent_version = None;
            model.use_responses_lite = false;
            model.comp_hash = None;
            // Avoid previous-model downshift compaction so this tests the selected provider.
            model.context_window = Some(1_000_000);
            model.auto_compact_token_limit = match compaction {
                Compaction::Automatic => Some(1),
                Compaction::Manual => None,
            };
        })
        .with_model("gpt-5.5")
        .with_config(move |config| {
            let _ = config.features.disable(Feature::TokenBudget);
            config.model_auto_compact_token_limit = match compaction {
                Compaction::Automatic => Some(1),
                Compaction::Manual => None,
            };
            config.composed_shim_model_slugs.insert(CUSTOM_MODEL.into());
            config.model_providers.insert(
                "codex_native_shim".into(),
                ModelProviderInfo {
                    name: "Quota-isolated custom provider".into(),
                    base_url: Some(custom_url),
                    ..ModelProviderInfo::default()
                },
            );
        });
    let test = builder.build_with_auto_env(&native).await?;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "native attempt".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    let mut native_errors = Vec::new();
    wait_for_event(&test.codex, |event| {
        if let EventMsg::Error(error) = event {
            native_errors.push(error.codex_error_info.clone());
        }
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert_eq!(
        native_errors,
        vec![Some(CodexErrorInfo::UsageLimitExceeded)]
    );

    test.codex
        .start_or_steer_turn(
            TurnInputRequest::user_input(vec![UserInput::Text {
                text: "continue on custom".into(),
                text_elements: Vec::new(),
            }])
            .with_thread_settings(ThreadSettingsOverrides {
                model: Some(CUSTOM_MODEL.into()),
                ..Default::default()
            }),
        )
        .await?;
    let mut custom_errors = Vec::new();
    wait_for_event(&test.codex, |event| {
        if let EventMsg::Error(error) = event {
            custom_errors.push(error.message.clone());
        }
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    match compaction {
        Compaction::Manual => {
            test.codex.submit(Op::Compact).await?;
        }
        Compaction::Automatic => {
            test.codex
                .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
                    text: "continue after crossing compact threshold".into(),
                    text_elements: Vec::new(),
                }]))
                .await?;
        }
    }
    wait_for_event(&test.codex, |event| {
        if let EventMsg::Error(error) = event {
            custom_errors.push(error.message.clone());
        }
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert_eq!(
        (
            custom_errors,
            native.received_requests().await.unwrap().len()
        ),
        (Vec::<String>::new(), 1)
    );
    assert_eq!(
        custom_requests
            .requests()
            .iter()
            .map(|request| request.body_json()["model"].clone())
            .collect::<Vec<_>>(),
        vec![json!(CUSTOM_MODEL); response_count]
    );
    assert_eq!(native.received_requests().await.unwrap().len(), 1);

    test.codex
        .start_or_steer_turn(
            TurnInputRequest::user_input(vec![UserInput::Text {
                text: "return to native".into(),
                text_elements: Vec::new(),
            }])
            .with_thread_settings(ThreadSettingsOverrides {
                model: Some("gpt-5.5".into()),
                ..Default::default()
            }),
        )
        .await?;
    native_errors.clear();
    wait_for_event(&test.codex, |event| {
        if let EventMsg::Error(error) = event {
            native_errors.push(error.codex_error_info.clone());
        }
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert_eq!(
        native_errors,
        vec![Some(CodexErrorInfo::UsageLimitExceeded)]
    );
    assert_eq!(native.received_requests().await.unwrap().len(), 2);
    Ok(())
}
