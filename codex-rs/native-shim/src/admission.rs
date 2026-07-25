use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use thiserror::Error;
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::Semaphore;

use crate::ConcurrencyPolicy;
use crate::NativeSettings;

/// Per-provider admission controller. A lease is held until the upstream
/// response stream completes, so every cancellation and normal error path
/// releases active capacity through `Drop`.
#[derive(Clone)]
pub struct AdmissionController {
    providers: HashMap<String, Arc<ProviderLimit>>,
}

impl AdmissionController {
    pub fn from_settings(settings: &NativeSettings) -> Result<Self, AdmissionError> {
        let mut providers: HashMap<String, Arc<ProviderLimit>> = HashMap::new();
        for route in settings.routes() {
            let provider_id = route.provider_id().to_owned();
            let policy = route.concurrency_policy();
            match providers.get(&provider_id) {
                Some(existing) if existing.policy() != policy => {
                    return Err(AdmissionError::ConflictingLimit { provider_id });
                }
                Some(_) => {}
                None => {
                    providers.insert(provider_id, Arc::new(ProviderLimit::new(policy.cloned())));
                }
            }
        }
        Ok(Self { providers })
    }

    pub async fn acquire(&self, provider_id: &str) -> Result<AdmissionLease, AdmissionError> {
        let provider = self
            .providers
            .get(provider_id)
            .ok_or_else(|| AdmissionError::UnknownProvider(provider_id.to_owned()))?;
        provider.acquire(provider_id).await
    }

    /// Returns bounded-provider capacity only; credentials and upstream URLs
    /// deliberately never cross this diagnostic boundary.
    pub fn diagnostics(&self, provider_id: &str) -> Option<ProviderAdmissionDiagnostics> {
        self.providers
            .get(provider_id)
            .and_then(|provider| provider.diagnostics())
    }
}

enum ProviderLimit {
    Unlimited,
    Limited {
        semaphore: Arc<Semaphore>,
        policy: ConcurrencyPolicy,
        queued: AtomicUsize,
    },
}

impl ProviderLimit {
    fn new(policy: Option<ConcurrencyPolicy>) -> Self {
        policy.map_or(Self::Unlimited, |policy| Self::Limited {
            semaphore: Arc::new(Semaphore::new(policy.max_active())),
            policy,
            queued: AtomicUsize::new(0),
        })
    }

    fn policy(&self) -> Option<&ConcurrencyPolicy> {
        match self {
            Self::Unlimited => None,
            Self::Limited { policy, .. } => Some(policy),
        }
    }

    async fn acquire(&self, provider_id: &str) -> Result<AdmissionLease, AdmissionError> {
        let Self::Limited {
            semaphore,
            policy,
            queued,
        } = self
        else {
            return Ok(AdmissionLease::Unlimited);
        };
        if let Ok(permit) = semaphore.clone().try_acquire_owned() {
            return Ok(AdmissionLease::Limited(permit));
        }
        let queue_slot = QueueSlot::claim(queued, policy.max_queued())
            .ok_or_else(|| AdmissionError::QueueFull(provider_id.to_owned()))?;
        let permit = tokio::time::timeout(policy.wait_timeout(), semaphore.clone().acquire_owned())
            .await
            .map_err(|_| AdmissionError::TimedOut(provider_id.to_owned()))?
            .map_err(|_| AdmissionError::ClosedProvider(provider_id.to_owned()))?;
        drop(queue_slot);
        Ok(AdmissionLease::Limited(permit))
    }

    fn diagnostics(&self) -> Option<ProviderAdmissionDiagnostics> {
        match self {
            Self::Unlimited => None,
            Self::Limited {
                semaphore,
                policy,
                queued,
            } => Some(ProviderAdmissionDiagnostics {
                max_active: policy.max_active(),
                active: policy.max_active() - semaphore.available_permits(),
                max_queued: policy.max_queued(),
                queued: queued.load(Ordering::Acquire),
            }),
        }
    }
}

struct QueueSlot<'a> {
    queued: &'a AtomicUsize,
}

impl<'a> QueueSlot<'a> {
    fn claim(queued: &'a AtomicUsize, max_queued: usize) -> Option<Self> {
        let mut current = queued.load(Ordering::Acquire);
        loop {
            if current >= max_queued {
                return None;
            }
            match queued.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(Self { queued }),
                Err(observed) => current = observed,
            }
        }
    }
}

impl Drop for QueueSlot<'_> {
    fn drop(&mut self) {
        self.queued.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Holds a provider's active request capacity until the response stream ends.
#[derive(Debug)]
pub enum AdmissionLease {
    Unlimited,
    Limited(OwnedSemaphorePermit),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderAdmissionDiagnostics {
    pub max_active: usize,
    pub active: usize,
    pub max_queued: usize,
    pub queued: usize,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum AdmissionError {
    #[error("native shim provider `{provider_id}` has conflicting concurrency limits")]
    ConflictingLimit { provider_id: String },
    #[error("native shim provider `{0}` is not configured")]
    UnknownProvider(String),
    #[error("native shim provider `{0}` queue is full")]
    QueueFull(String),
    #[error("native shim provider `{0}` did not admit the request before its deadline")]
    TimedOut(String),
    #[error("native shim provider `{0}` closed while waiting for capacity")]
    ClosedProvider(String),
}
