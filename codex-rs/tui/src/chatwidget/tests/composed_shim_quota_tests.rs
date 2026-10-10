use super::*;
use codex_app_server_protocol::GetAccountRateLimitsResponse;
use pretty_assertions::assert_eq;
use serde_json::json;

#[tokio::test]
async fn composed_shim_quota_does_not_inherit_native_reserve_gate() {
    let custom_model = "custom/quota-isolated";
    let (mut chat, _events, _ops) = make_chatwidget_manual(Some(custom_model)).await;
    chat.has_chatgpt_account = true;
    chat.requires_openai_auth = true;
    chat.config
        .composed_shim_model_slugs
        .insert(custom_model.into());
    let template = chat.model_catalog.try_list_models().unwrap()[0].clone();
    chat.model_catalog = Arc::new(ModelCatalog::new(
        [custom_model, crate::model_catalog::LUNA_RESERVE_MODEL]
            .map(|model| ModelPreset {
                model: model.into(),
                ..template.clone()
            })
            .to_vec(),
    ));
    let response: GetAccountRateLimitsResponse = serde_json::from_value(json!({
        "accountId": "native-account",
        "ordinaryUsageAllowed": false,
        "rateLimits": {},
        "rateLimitUpsell": {
            "banner_type": "luna_reserve",
            "presentation": "dismissible",
            "title": "Native quota exhausted",
            "description": "Native usage requires Reserve.",
            "ctas": [{"action": "add_credits", "label": "Add credits"}]
        }
    }))
    .unwrap();
    chat.update_backend_banner(&response);

    assert_eq!(
        (
            chat.backend_banner_fallback(),
            chat.waiting_for_luna_reserve(),
            chat.has_applicable_backend_banner()
        ),
        (None, false, false)
    );
    insta::assert_snapshot!(
        "composed_shim_quota_ignores_native_reserve_banner",
        normalize_snapshot_paths(render_bottom_popup(&chat, /*width*/ 90))
    );
}
