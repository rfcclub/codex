use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;
use url::Url;

const DEFAULT_MAX_QUEUED: usize = 16;
const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpstreamProtocol {
    OpenAiResponses,
    OpenAiChatCompletions,
    AnthropicMessages,
}

impl UpstreamProtocol {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "openai" | "generic-chat-completion-api" | "openai-chat-completions" => {
                Some(Self::OpenAiChatCompletions)
            }
            "openai-responses" => Some(Self::OpenAiResponses),
            "anthropic" | "anthropic-messages" => Some(Self::AnthropicMessages),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcurrencyPolicy {
    max_active: usize,
    max_queued: usize,
    wait_timeout: Duration,
}

impl ConcurrencyPolicy {
    pub const fn new(max_active: usize, max_queued: usize, wait_timeout: Duration) -> Self {
        Self {
            max_active,
            max_queued,
            wait_timeout,
        }
    }

    pub const fn max_active(&self) -> usize {
        self.max_active
    }

    pub const fn max_queued(&self) -> usize {
        self.max_queued
    }

    pub const fn wait_timeout(&self) -> Duration {
        self.wait_timeout
    }
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("invalid native shim settings JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("native shim settings must contain at least one model")]
    NoModels,
    #[error("native shim model at index {index} is missing `{field}")]
    MissingField { index: usize, field: &'static str },
    #[error("duplicate native shim model slug `{0}`")]
    DuplicateSlug(String),
    #[error("unsupported native shim provider `{provider}` for model `{model}`")]
    UnsupportedProvider { provider: String, model: String },
    #[error("native shim model `{model}` references unknown provider `{provider_id}`")]
    UnknownProvider { model: String, provider_id: String },
    #[error("invalid native shim base URL for model `{model}`: {source}")]
    InvalidBaseUrl {
        model: String,
        source: url::ParseError,
    },
    #[error("native shim base URL for model `{model}` must use http or https")]
    UnsupportedBaseUrlScheme { model: String },
    #[error("native shim concurrency limit for model `{model}` must be positive")]
    InvalidConcurrencyLimit { model: String },
    #[error("native shim queue capacity for provider `{provider_id}` must be positive")]
    InvalidQueueCapacity { provider_id: String },
    #[error("native shim wait timeout for provider `{provider_id}` must be positive")]
    InvalidWaitTimeout { provider_id: String },
    #[error("native shim provider `{provider_id}` is missing an API key")]
    MissingApiKey { provider_id: String },
}

/// Validated, read-only native-shim configuration.
///
/// The API key remains private so diagnostics and debug rendering cannot expose
/// it accidentally. The relay consumes it inside this crate.
pub struct NativeSettings {
    routes: Vec<NativeRoute>,
}

impl fmt::Debug for NativeSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeSettings")
            .field("route_count", &self.routes.len())
            .finish()
    }
}

impl NativeSettings {
    pub fn from_json(input: &str) -> Result<Self, SettingsError> {
        Self::from_json_with_env(input, |name| std::env::var(name).ok())
    }

    pub fn from_json_with_env(
        input: &str,
        lookup_env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, SettingsError> {
        let raw = serde_json::from_str::<RawSettings>(input)?;
        if raw.models.is_empty() {
            return Err(SettingsError::NoModels);
        }
        if raw.providers.is_empty() {
            Self::from_legacy(raw.models)
        } else {
            Self::from_preferred(raw, lookup_env)
        }
    }

    fn from_legacy(models: Vec<RawRoute>) -> Result<Self, SettingsError> {
        let mut slugs = HashSet::new();
        let mut routes = Vec::with_capacity(models.len());
        for (index, route) in models.into_iter().enumerate() {
            let model = required(route.model, index, "model")?;
            let provider = required(route.provider, index, "provider")?;
            let protocol = UpstreamProtocol::parse(&provider).ok_or_else(|| {
                SettingsError::UnsupportedProvider {
                    provider: provider.clone(),
                    model: model.clone(),
                }
            })?;
            let provider_id = route
                .provider_entry_id
                .map(|entry_id| entry_id.trim().to_owned())
                .filter(|entry_id| !entry_id.is_empty())
                .unwrap_or(provider);
            let slug = unique_slug(route.slug, &model, index, &mut slugs)?;
            let base_url = parse_base_url(required(route.base_url, index, "base_url")?, &model)?;
            let api_key = required(route.api_key, index, "api_key")?;
            let concurrency = route.provider_concurrency_limit.map(|max_active| {
                ConcurrencyPolicy::new(max_active, DEFAULT_MAX_QUEUED, DEFAULT_WAIT_TIMEOUT)
            });
            if concurrency
                .as_ref()
                .is_some_and(|policy| policy.max_active == 0)
            {
                return Err(SettingsError::InvalidConcurrencyLimit { model });
            }
            routes.push(NativeRoute {
                model,
                slug,
                provider_id,
                protocol,
                base_url,
                api_key,
                concurrency,
            });
        }
        Ok(Self { routes })
    }

    fn from_preferred(
        raw: RawSettings,
        lookup_env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, SettingsError> {
        let providers = raw
            .providers
            .into_iter()
            .map(|provider| {
                let id = required(provider.id.clone(), 0, "providers[].id")?;
                Ok((id, provider))
            })
            .collect::<Result<HashMap<_, _>, SettingsError>>()?;
        let mut slugs = HashSet::new();
        let mut routes = Vec::with_capacity(raw.models.len());
        for (index, route) in raw.models.into_iter().enumerate() {
            let model = required(
                route.upstream_model.or(route.model),
                index,
                "upstream_model",
            )?;
            let slug = unique_slug(route.slug, &model, index, &mut slugs)?;
            let provider_id = required(route.provider, index, "provider")?;
            let provider =
                providers
                    .get(&provider_id)
                    .ok_or_else(|| SettingsError::UnknownProvider {
                        model: model.clone(),
                        provider_id: provider_id.clone(),
                    })?;
            let protocol_name = required(provider.protocol.clone(), index, "providers[].protocol")?;
            let protocol = UpstreamProtocol::parse(&protocol_name).ok_or_else(|| {
                SettingsError::UnsupportedProvider {
                    provider: protocol_name,
                    model: model.clone(),
                }
            })?;
            let base_url = parse_base_url(
                required(provider.base_url.clone(), index, "providers[].base_url")?,
                &model,
            )?;
            let api_key = resolve_api_key(provider, &provider_id, &lookup_env)?;
            let concurrency = provider
                .concurrency
                .as_ref()
                .map(|raw| concurrency_policy(raw, &provider_id))
                .transpose()?;
            routes.push(NativeRoute {
                model,
                slug,
                provider_id,
                protocol,
                base_url,
                api_key,
                concurrency,
            });
        }
        Ok(Self { routes })
    }

    pub fn routes(&self) -> &[NativeRoute] {
        &self.routes
    }
}

pub struct NativeRoute {
    model: String,
    slug: String,
    provider_id: String,
    protocol: UpstreamProtocol,
    base_url: Url,
    api_key: String,
    concurrency: Option<ConcurrencyPolicy>,
}

impl NativeRoute {
    pub fn model(&self) -> &str {
        &self.model
    }
    pub fn slug(&self) -> &str {
        &self.slug
    }
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }
    pub fn protocol(&self) -> UpstreamProtocol {
        self.protocol
    }
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }
    pub fn concurrency_limit(&self) -> Option<usize> {
        self.concurrency.as_ref().map(ConcurrencyPolicy::max_active)
    }
    pub fn concurrency_policy(&self) -> Option<&ConcurrencyPolicy> {
        self.concurrency.as_ref()
    }
    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
    }
    pub(crate) fn api_key(&self) -> &str {
        &self.api_key
    }
}

#[derive(Deserialize)]
struct RawSettings {
    #[serde(default)]
    providers: Vec<RawProvider>,
    #[serde(
        default,
        alias = "customModels",
        alias = "launchModels",
        alias = "launch_models"
    )]
    models: Vec<RawRoute>,
}

#[derive(Deserialize)]
struct RawRoute {
    model: Option<String>,
    slug: Option<String>,
    provider: Option<String>,
    #[serde(alias = "providerEntryId")]
    provider_entry_id: Option<String>,
    #[serde(alias = "upstreamModel")]
    upstream_model: Option<String>,
    #[serde(alias = "baseUrl", alias = "baseURL")]
    base_url: Option<String>,
    #[serde(alias = "apiKey", alias = "bearerToken")]
    api_key: Option<String>,
    #[serde(default, alias = "concurrency_limit")]
    provider_concurrency_limit: Option<usize>,
}

#[derive(Deserialize)]
struct RawProvider {
    id: Option<String>,
    protocol: Option<String>,
    #[serde(alias = "baseUrl")]
    base_url: Option<String>,
    #[serde(alias = "apiKey")]
    api_key: Option<String>,
    #[serde(alias = "apiKeyEnv")]
    api_key_env: Option<String>,
    concurrency: Option<RawConcurrency>,
}

#[derive(Deserialize)]
struct RawConcurrency {
    max_active: Option<usize>,
    max_queued: Option<usize>,
    wait_timeout_ms: Option<u64>,
}

fn unique_slug(
    slug: Option<String>,
    model: &str,
    index: usize,
    slugs: &mut HashSet<String>,
) -> Result<String, SettingsError> {
    let slug = slug.unwrap_or_else(|| model.to_owned()).trim().to_owned();
    if slug.is_empty() {
        return Err(SettingsError::MissingField {
            index,
            field: "slug",
        });
    }
    if !slugs.insert(slug.clone()) {
        return Err(SettingsError::DuplicateSlug(slug));
    }
    Ok(slug)
}

fn parse_base_url(value: String, model: &str) -> Result<Url, SettingsError> {
    let mut base_url = Url::parse(&value).map_err(|source| SettingsError::InvalidBaseUrl {
        model: model.to_owned(),
        source,
    })?;
    if !matches!(base_url.scheme(), "http" | "https") {
        return Err(SettingsError::UnsupportedBaseUrlScheme {
            model: model.to_owned(),
        });
    }
    let normalized_path = base_url.path().trim_end_matches('/').to_owned();
    base_url.set_path(&normalized_path);
    Ok(base_url)
}

fn resolve_api_key(
    provider: &RawProvider,
    provider_id: &str,
    lookup_env: &impl Fn(&str) -> Option<String>,
) -> Result<String, SettingsError> {
    if let Some(api_key) = provider
        .api_key
        .clone()
        .filter(|key| !key.trim().is_empty())
    {
        return Ok(api_key);
    }
    if let Some(name) = provider
        .api_key_env
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        && let Some(api_key) = lookup_env(name).filter(|key| !key.trim().is_empty())
    {
        return Ok(api_key);
    }
    Err(SettingsError::MissingApiKey {
        provider_id: provider_id.to_owned(),
    })
}

fn concurrency_policy(
    raw: &RawConcurrency,
    provider_id: &str,
) -> Result<ConcurrencyPolicy, SettingsError> {
    let max_active = raw.max_active.unwrap_or(1);
    if max_active == 0 {
        return Err(SettingsError::InvalidConcurrencyLimit {
            model: provider_id.to_owned(),
        });
    }
    let max_queued = raw.max_queued.unwrap_or(DEFAULT_MAX_QUEUED);
    if max_queued == 0 {
        return Err(SettingsError::InvalidQueueCapacity {
            provider_id: provider_id.to_owned(),
        });
    }
    let wait_timeout_ms = raw
        .wait_timeout_ms
        .unwrap_or(DEFAULT_WAIT_TIMEOUT.as_millis() as u64);
    if wait_timeout_ms == 0 {
        return Err(SettingsError::InvalidWaitTimeout {
            provider_id: provider_id.to_owned(),
        });
    }
    Ok(ConcurrencyPolicy::new(
        max_active,
        max_queued,
        Duration::from_millis(wait_timeout_ms),
    ))
}

fn required(
    value: Option<String>,
    index: usize,
    field: &'static str,
) -> Result<String, SettingsError> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or(SettingsError::MissingField { index, field })
}
