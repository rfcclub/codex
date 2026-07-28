use super::composed_catalog::ComposedModelsManager;
use crate::manager::ModelsManager;
use crate::manager::RefreshStrategy;
use crate::manager::StaticModelsManager;
use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;
use codex_protocol::openai_models::ModelsResponse;
use pretty_assertions::assert_eq;
use serde_json::json;

const HTTP: HttpClientFactory = HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault);

fn catalog(slugs: &[&str]) -> ModelsResponse {
    serde_json::from_value(json!({
        "models": slugs.iter().enumerate().map(|(priority, slug)| json!({
            "slug": slug,
            "display_name": slug,
            "description": null,
            "default_reasoning_level": "medium",
            "supported_reasoning_levels": [],
            "shell_type": "shell_command",
            "visibility": "list",
            "supported_in_api": true,
            "priority": priority,
            "availability_nux": null,
            "upgrade": null,
            "base_instructions": "test",
            "support_verbosity": false,
            "default_verbosity": null,
            "truncation_policy": {"mode": "tokens", "limit": 64000},
            "supports_parallel_tool_calls": true,
            "experimental_supported_tools": []
        })).collect::<Vec<_>>()
    }))
    .expect("valid test catalog")
}

#[tokio::test]
async fn composes_native_and_shim_models_without_replacing_native_catalog() {
    let native = StaticModelsManager::new(None, catalog(&["native-model"]));
    let composed = ComposedModelsManager::new(native, catalog(&["shim-model"]));

    let actual = composed
        .raw_model_catalog(RefreshStrategy::Offline, HTTP)
        .await;
    let slugs = actual
        .models
        .into_iter()
        .map(|model| model.slug)
        .collect::<Vec<_>>();
    assert_eq!(slugs, vec!["native-model", "shim-model"]);
}

#[tokio::test]
async fn prefers_shim_model_for_cross_source_slug_collisions() {
    let mut remote_catalog = catalog(&["crof/glm-5.2", "gpt-5.6"]);
    remote_catalog.models[0].display_name = "Remote GLM".to_string();
    let mut shim_catalog = catalog(&["crof/glm-5.2"]);
    shim_catalog.models[0].display_name = "Shim GLM".to_string();
    let expected = ModelsResponse {
        models: vec![
            shim_catalog.models[0].clone(),
            remote_catalog.models[1].clone(),
        ],
    };
    let native = StaticModelsManager::new(None, remote_catalog);
    let composed = ComposedModelsManager::new(native, shim_catalog);

    let actual = composed
        .raw_model_catalog(RefreshStrategy::Offline, HTTP)
        .await;
    assert_eq!(actual, expected);
}
