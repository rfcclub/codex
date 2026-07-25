use std::time::Duration;

use pretty_assertions::assert_eq;
use tokio::sync::mpsc;

use super::AdmissionController;
use super::AdmissionError;
use super::NativeSettings;

#[tokio::test]
async fn saturated_provider_does_not_block_another_provider() {
    let settings = NativeSettings::from_json(
        r#"{
            "models": [
                {"model": "cheap", "provider": "openai", "base_url": "https://cheap.example/v1", "api_key": "a", "provider_concurrency_limit": 1},
                {"model": "fast", "provider": "generic-chat-completion-api", "base_url": "https://fast.example/v1", "api_key": "b", "provider_concurrency_limit": 1}
            ]
        }"#,
    )
    .expect("valid settings");
    let admission = AdmissionController::from_settings(&settings).expect("valid limits");

    let cheap_lease = admission
        .acquire("openai")
        .await
        .expect("first cheap lease");
    assert!(
        tokio::time::timeout(Duration::from_millis(10), admission.acquire("openai"))
            .await
            .is_err(),
        "a second cheap request must remain queued"
    );
    let fast_lease = tokio::time::timeout(
        Duration::from_millis(10),
        admission.acquire("generic-chat-completion-api"),
    )
    .await
    .expect("unrelated provider must not wait")
    .expect("known provider");

    drop(cheap_lease);
    drop(fast_lease);
}

#[tokio::test]
async fn rejects_a_request_when_the_provider_queue_is_full() {
    let admission = controller_with_policy(1, 1, 1000);
    let active = admission.acquire("primary").await.expect("active lease");
    let queued = tokio::spawn({
        let admission = admission.clone();
        async move { admission.acquire("primary").await }
    });
    tokio::task::yield_now().await;

    let error = admission
        .acquire("primary")
        .await
        .expect_err("the bounded queue must reject overflow");
    assert_eq!(error, AdmissionError::QueueFull("primary".to_owned()));

    drop(active);
    drop(
        queued
            .await
            .expect("queued task joins")
            .expect("queued lease"),
    );
}

#[tokio::test]
async fn releases_queued_requests_in_fifo_order() {
    let admission = controller_with_policy(1, 2, 1000);
    let active = admission.acquire("primary").await.expect("active lease");
    let (started_tx, mut started_rx) = mpsc::channel(2);
    let (finished_tx, mut finished_rx) = mpsc::channel(2);
    for request_id in [1, 2] {
        let admission = admission.clone();
        let started_tx = started_tx.clone();
        let finished_tx = finished_tx.clone();
        tokio::spawn(async move {
            started_tx.send(request_id).await.expect("start signal");
            let lease = admission.acquire("primary").await.expect("queued lease");
            finished_tx.send(request_id).await.expect("finish signal");
            drop(lease);
        });
        assert_eq!(started_rx.recv().await, Some(request_id));
        tokio::task::yield_now().await;
    }

    drop(active);
    assert_eq!(finished_rx.recv().await, Some(1));
    assert_eq!(finished_rx.recv().await, Some(2));
}

#[tokio::test]
async fn times_out_and_releases_the_queue_slot() {
    let admission = controller_with_policy(1, 1, 5);
    let active = admission.acquire("primary").await.expect("active lease");

    let error = admission
        .acquire("primary")
        .await
        .expect_err("saturated provider must time out");
    assert_eq!(error, AdmissionError::TimedOut("primary".to_owned()));

    drop(active);
    drop(
        admission
            .acquire("primary")
            .await
            .expect("released capacity"),
    );
}

#[tokio::test]
async fn cancelling_a_queued_request_returns_its_queue_slot() {
    let admission = controller_with_policy(1, 1, 1000);
    let active = admission.acquire("primary").await.expect("active lease");
    let cancelled = tokio::spawn({
        let admission = admission.clone();
        async move { admission.acquire("primary").await }
    });
    tokio::task::yield_now().await;
    cancelled.abort();
    let _ = cancelled.await;

    let replacement = tokio::spawn({
        let admission = admission.clone();
        async move { admission.acquire("primary").await }
    });
    tokio::task::yield_now().await;
    drop(active);
    drop(
        replacement
            .await
            .expect("replacement joins")
            .expect("replacement lease"),
    );
}

#[tokio::test]
async fn dropping_a_lease_after_an_upstream_error_releases_capacity_once() {
    let admission = controller_with_policy(1, 1, 1000);
    let failed_request = admission.acquire("primary").await.expect("active lease");
    let next_request = tokio::spawn({
        let admission = admission.clone();
        async move { admission.acquire("primary").await }
    });
    tokio::task::yield_now().await;

    drop(failed_request);
    let next_request = next_request
        .await
        .expect("next request joins")
        .expect("next lease");
    drop(next_request);
    drop(
        admission
            .acquire("primary")
            .await
            .expect("exactly one permit returned"),
    );
}

#[tokio::test]
async fn diagnostics_report_capacity_without_exposing_provider_secrets() {
    let admission = controller_with_policy(1, 2, 1000);
    let active = admission.acquire("primary").await.expect("active lease");
    assert_eq!(
        admission.diagnostics("primary"),
        Some(super::ProviderAdmissionDiagnostics {
            max_active: 1,
            active: 1,
            max_queued: 2,
            queued: 0,
        })
    );
    drop(active);
}

fn controller_with_policy(
    max_active: usize,
    max_queued: usize,
    wait_timeout_ms: u64,
) -> AdmissionController {
    let settings = NativeSettings::from_json(&format!(
        r#"{{
            "providers": [{{
                "id": "primary",
                "protocol": "openai-responses",
                "base_url": "https://primary.example/v1",
                "api_key": "secret",
                "concurrency": {{ "max_active": {max_active}, "max_queued": {max_queued}, "wait_timeout_ms": {wait_timeout_ms} }}
            }}],
            "models": [{{ "slug": "primary", "upstream_model": "primary", "provider": "primary" }}]
        }}"#
    ))
    .expect("valid settings");
    AdmissionController::from_settings(&settings).expect("valid controller")
}
