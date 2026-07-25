use codex_protocol::openai_models::ModelsResponse;
use serde_json::json;

use crate::NativeSettings;

pub fn catalog_for_settings(
    settings: &NativeSettings,
) -> Result<ModelsResponse, serde_json::Error> {
    let models = settings
        .routes()
        .iter()
        .enumerate()
        .map(|(index, route)| {
            json!({
                "slug": route.slug(),
                "display_name": route.slug(),
                "description": format!("{} via native Codex shim.", route.slug()),
                "default_reasoning_level": "medium",
                "supported_reasoning_levels": [
                    { "effort": "low", "description": "Faster, lighter reasoning" },
                    { "effort": "medium", "description": "Balanced speed and reasoning" },
                    { "effort": "high", "description": "Deeper reasoning" }
                ],
                "shell_type": "shell_command",
                "visibility": "list",
                "supported_in_api": true,
                "priority": 1000_i32 - index as i32,
                "availability_nux": null,
                "upgrade": null,
                "base_instructions": "You are Codex running through a local native model shim.",
                "support_verbosity": false,
                "default_verbosity": "low",
                "apply_patch_tool_type": "freeform",
                "web_search_tool_type": "text_and_image",
                "truncation_policy": { "mode": "tokens", "limit": 64000 },
                "supports_parallel_tool_calls": true,
                "context_window": 128000,
                "max_context_window": 128000,
                "auto_compact_token_limit": 102400,
                "experimental_supported_tools": [],
                "input_modalities": ["text", "image"],
                "supports_search_tool": false
            })
        })
        .collect::<Vec<_>>();
    serde_json::from_value(json!({ "models": models }))
}
