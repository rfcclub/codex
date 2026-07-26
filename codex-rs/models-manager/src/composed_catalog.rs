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
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::TryLockError;
use tracing::warn;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposedCatalogError {
    slug: String,
}

impl fmt::Display for ComposedCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "duplicate composed model slug `{}`", self.slug)
    }
}

impl std::error::Error for ComposedCatalogError {}

impl ComposedModelsManager {
    pub fn new<M>(native: M, shim_catalog: ModelsResponse) -> Result<Self, ComposedCatalogError>
    where
        M: ModelsManager + 'static,
    {
        Self::from_shared(Arc::new(native), shim_catalog)
    }

    pub fn from_shared(
        native: SharedModelsManager,
        shim_catalog: ModelsResponse,
    ) -> Result<Self, ComposedCatalogError> {
        let shim_models = shim_catalog.models;
        let initial = merge_models(
            native.try_get_remote_models().unwrap_or_default(),
            &shim_models,
        )?;
        Ok(Self {
            native,
            shim_models,
            last_valid_models: RwLock::new(initial),
        })
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
        match merge_models(native.models, &self.shim_models) {
            Ok(models) => {
                *self.last_valid_models.write().await = models.clone();
                models
            }
            Err(error) => {
                warn!(%error, "keeping last valid composed model catalog after native refresh");
                self.last_valid_models.read().await.clone()
            }
        }
    }
}

fn merge_models(
    mut native_models: Vec<ModelInfo>,
    shim_models: &[ModelInfo],
) -> Result<Vec<ModelInfo>, ComposedCatalogError> {
    let mut slugs = HashSet::with_capacity(native_models.len() + shim_models.len());
    for model in &native_models {
        if !slugs.insert(model.slug.clone()) {
            return Err(ComposedCatalogError {
                slug: model.slug.clone(),
            });
        }
    }
    for model in shim_models {
        if !slugs.insert(model.slug.clone()) {
            return Err(ComposedCatalogError {
                slug: model.slug.clone(),
            });
        }
        native_models.push(model.clone());
    }
    native_models.sort_by(|left, right| left.slug.cmp(&right.slug));
    Ok(native_models)
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
