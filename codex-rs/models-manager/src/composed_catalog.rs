use crate::manager::ModelsManager;
use crate::manager::ModelsManagerFuture;
use crate::manager::RefreshStrategy;
use crate::manager::SharedModelsManager;
use codex_http_client::HttpClientFactory;
use codex_login::AuthManager;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelsResponse;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::TryLockError;

/// An immutable shim extension layered over the normal authenticated manager.
///
/// The native manager remains responsible for auth filtering and refresh. Each
/// successful native snapshot is merged with the validated shim entries before
/// it becomes visible to callers.
#[derive(Debug)]
pub struct ComposedModelsManager {
    native: SharedModelsManager,
    shim_models: Vec<ModelInfo>,
    last_valid_models: RwLock<Vec<ModelInfo>>,
}

impl ComposedModelsManager {
    pub fn new<M>(native: M, shim_catalog: ModelsResponse) -> Self
    where
        M: ModelsManager + 'static,
    {
        Self::from_shared(Arc::new(native), shim_catalog)
    }

    pub fn from_shared(native: SharedModelsManager, shim_catalog: ModelsResponse) -> Self {
        let shim_models = shim_catalog.models;
        let initial = merge_models(
            native.try_get_remote_models().unwrap_or_default(),
            &shim_models,
        );
        Self {
            native,
            shim_models,
            last_valid_models: RwLock::new(initial),
        }
    }

    async fn refresh_composed_catalog(
        &self,
        refresh_strategy: RefreshStrategy,
        http_client_factory: HttpClientFactory,
    ) -> Vec<ModelInfo> {
        let native = self
            .native
            .raw_model_catalog(refresh_strategy, http_client_factory)
            .await;
        let models = merge_models(native.models, &self.shim_models);
        *self.last_valid_models.write().await = models.clone();
        models
    }
}

fn merge_models(mut native_models: Vec<ModelInfo>, shim_models: &[ModelInfo]) -> Vec<ModelInfo> {
    let shim_slugs = shim_models
        .iter()
        .map(|shim_model| shim_model.slug.as_str())
        .collect::<HashSet<_>>();
    native_models.retain(|native_model| !shim_slugs.contains(native_model.slug.as_str()));
    native_models.extend_from_slice(shim_models);
    native_models.sort_by(|left, right| left.slug.cmp(&right.slug));
    native_models
}

impl ModelsManager for ComposedModelsManager {
    fn raw_model_catalog(
        &self,
        refresh_strategy: RefreshStrategy,
        http_client_factory: HttpClientFactory,
    ) -> ModelsManagerFuture<'_, ModelsResponse> {
        Box::pin(async move {
            ModelsResponse {
                models: self
                    .refresh_composed_catalog(refresh_strategy, http_client_factory)
                    .await,
            }
        })
    }

    fn get_remote_models(&self) -> ModelsManagerFuture<'_, Vec<ModelInfo>> {
        Box::pin(async move { self.last_valid_models.read().await.clone() })
    }

    fn try_get_remote_models(&self) -> Result<Vec<ModelInfo>, TryLockError> {
        Ok(self.last_valid_models.try_read()?.clone())
    }

    fn auth_manager(&self) -> Option<&AuthManager> {
        self.native.auth_manager()
    }

    fn list_collaboration_modes(&self) -> Vec<CollaborationModeMask> {
        self.native.list_collaboration_modes()
    }

    fn refresh_if_new_etag(
        &self,
        etag: String,
        http_client_factory: HttpClientFactory,
    ) -> ModelsManagerFuture<'_, ()> {
        Box::pin(async move {
            self.native
                .refresh_if_new_etag(etag, http_client_factory.clone())
                .await;
            let _ = self
                .refresh_composed_catalog(RefreshStrategy::Offline, http_client_factory)
                .await;
        })
    }
}
