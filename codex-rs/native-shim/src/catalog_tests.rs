use pretty_assertions::assert_eq;

use super::NativeSettings;
use super::catalog_for_settings;

#[test]
fn generates_a_codex_compatible_catalog_for_every_route() {
    let settings = NativeSettings::from_json(
        r#"{"models":[{"model":"sonnet","slug":"cheap-sonnet","provider":"openai","base_url":"https://example.test/v1","api_key":"secret"}]}"#,
    )
    .expect("settings");

    let catalog = catalog_for_settings(&settings).expect("Codex-compatible catalog");

    assert_eq!(catalog.models.len(), 1);
    assert_eq!(catalog.models[0].slug, "cheap-sonnet");
    assert_eq!(catalog.models[0].display_name, "cheap-sonnet");
    assert!(catalog.models[0].supported_in_api);
}
