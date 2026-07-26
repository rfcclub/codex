use anyhow::Result;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[test]
fn shim_launches_the_current_binary_without_relying_on_path() -> Result<()> {
    let workspace = TempDir::new()?;
    let codex_home = workspace.path().join("codex-home");
    let runtime_temp = workspace.path().join("runtime-temp");
    std::fs::create_dir_all(&codex_home)?;
    std::fs::create_dir_all(&runtime_temp)?;
    let settings_path = workspace.path().join("models.json");
    std::fs::write(
        &settings_path,
        r#"{
            "models": [{
                "model": "upstream-test-model",
                "slug": "local-test-model",
                "provider": "openai-responses",
                "base_url": "http://127.0.0.1:9/v1",
                "api_key": "test-only"
            }]
        }"#,
    )?;

    let mut command = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("codex")?);
    let output = command
        .env("CODEX_HOME", codex_home)
        .env("TMPDIR", &runtime_temp)
        .env("PATH", "/definitely-not-a-real-path")
        .args(["shim", "--settings"])
        .arg(&settings_path)
        .args(["--", "--version"])
        .output()?;

    assert!(
        output.status.success(),
        "shim failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("codex"));
    assert!(
        std::fs::read_dir(&runtime_temp)?.next().is_none(),
        "shim leaked a private runtime directory after the child exited"
    );
    Ok(())
}

#[test]
fn shim_doctor_validates_settings_without_launching_a_child() -> Result<()> {
    let workspace = TempDir::new()?;
    let codex_home = workspace.path().join("codex-home");
    std::fs::create_dir_all(&codex_home)?;
    let settings_path = workspace.path().join("models.json");
    std::fs::write(
        &settings_path,
        r#"{
            "providers": [{
                "id": "primary",
                "protocol": "openai-responses",
                "base_url": "http://127.0.0.1:9/v1",
                "api_key": "test-only"
            }],
            "models": [{
                "slug": "local-test-model",
                "upstream_model": "upstream-test-model",
                "provider": "primary"
            }]
        }"#,
    )?;

    let mut command = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("codex")?);
    let output = command
        .env("CODEX_HOME", workspace.path().join("codex-home"))
        .env("CODEX_HOME", codex_home)
        .args(["shim", "--settings"])
        .arg(&settings_path)
        .arg("doctor")
        .output()?;

    assert!(
        output.status.success(),
        "doctor failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("settings valid"));
    assert!(stdout.contains("preferred provider/model schema"));
    assert!(!stdout.contains("test-only"));
    Ok(())
}

#[test]
fn shim_exposes_every_configured_model_to_the_child_catalog() -> Result<()> {
    let workspace = TempDir::new()?;
    let codex_home = workspace.path().join("codex-home");
    std::fs::create_dir_all(&codex_home)?;
    let settings_path = workspace.path().join("models.json");
    std::fs::write(
        &settings_path,
        r#"{
            "providers": [{
                "id": "primary",
                "protocol": "openai-responses",
                "base_url": "http://127.0.0.1:9/v1",
                "api_key": "test-only"
            }],
            "models": [{
                "slug": "local-test-model",
                "upstream_model": "upstream-test-model",
                "provider": "primary"
            }, {
                "slug": "local-second-model",
                "upstream_model": "upstream-second-model",
                "provider": "primary"
            }]
        }"#,
    )?;

    let mut command = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("codex")?);
    let output = command
        .env("CODEX_HOME", codex_home)
        .args(["shim", "--settings"])
        .arg(&settings_path)
        .args(["--", "debug", "models"])
        .output()?;

    assert!(
        output.status.success(),
        "shim failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("local-test-model"));
    assert!(stdout.contains("local-second-model"));
    assert!(!stdout.contains("test-only"));
    Ok(())
}

#[test]
fn compose_keeps_native_catalog_and_adds_shim_models_without_exposing_a_key() -> Result<()> {
    let workspace = TempDir::new()?;
    let codex_home = workspace.path().join("codex-home");
    std::fs::create_dir_all(&codex_home)?;
    let settings_path = workspace.path().join("models.json");
    std::fs::write(
        &settings_path,
        r#"{
            "providers": [{
                "id": "private",
                "protocol": "openai-responses",
                "base_url": "http://127.0.0.1:9/v1",
                "api_key": "compose-test-secret"
            }],
            "models": [{
                "slug": "composed-shim-model",
                "upstream_model": "upstream-private-model",
                "provider": "private"
            }]
        }"#,
    )?;

    let mut command = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("codex")?);
    let output = command
        .env("CODEX_HOME", codex_home)
        .args(["shim", "--compose", "--settings"])
        .arg(&settings_path)
        .args(["--", "debug", "models"])
        .output()?;

    assert!(
        output.status.success(),
        "composed shim failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("composed-shim-model"));
    assert!(!stdout.contains("compose-test-secret"));
    assert!(!stdout.contains("upstream-private-model"));
    Ok(())
}

#[test]
fn shim_forwards_a_child_exit_status() -> Result<()> {
    let workspace = TempDir::new()?;
    let settings_path = workspace.path().join("models.json");
    std::fs::write(
        &settings_path,
        r#"{
            "models": [{
                "model": "upstream-test-model",
                "slug": "local-test-model",
                "provider": "openai-responses",
                "base_url": "http://127.0.0.1:9/v1",
                "api_key": "test-only"
            }]
        }"#,
    )?;

    let mut command = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("codex")?);
    let output = command
        .env("CODEX_HOME", workspace.path().join("codex-home"))
        .args(["shim", "--settings"])
        .arg(&settings_path)
        .args(["--", "--not-a-codex-argument"])
        .output()?;

    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shim_routes_a_child_responses_request_to_the_configured_upstream() -> Result<()> {
    let upstream = MockServer::start().await;
    let upstream_sse = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-shim-e2e\"}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-shim-e2e\",\"usage\":{\"input_tokens\":0,\"input_tokens_details\":null,\"output_tokens\":0,\"output_tokens_details\":null,\"total_tokens\":0}}}\n\n",
    );
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(upstream_sse),
        )
        .expect(1)
        .mount(&upstream)
        .await;

    let workspace = TempDir::new()?;
    let settings_path = workspace.path().join("models.json");
    std::fs::write(
        &settings_path,
        format!(
            r#"{{
                "providers": [{{
                    "id": "upstream",
                    "protocol": "openai-responses",
                    "base_url": "{}/v1",
                    "api_key": "test-only-key"
                }}],
                "models": [{{
                    "slug": "shim-e2e-model",
                    "upstream_model": "actual-upstream-model",
                    "provider": "upstream"
                }}]
            }}"#,
            upstream.uri()
        ),
    )?;

    let binary = codex_utils_cargo_bin::cargo_bin("codex")?;
    let codex_home = workspace.path().join("codex-home");
    std::fs::create_dir_all(&codex_home)?;
    let output = tokio::task::spawn_blocking(move || {
        assert_cmd::Command::new(binary)
            .env("CODEX_HOME", codex_home)
            .args(["shim", "--settings"])
            .arg(&settings_path)
            .args([
                "--",
                "exec",
                "--skip-git-repo-check",
                "--experimental-json",
                "return the fixture response",
            ])
            .output()
    })
    .await??;

    assert!(
        output.status.success(),
        "shim e2e failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = upstream
        .received_requests()
        .await
        .expect("wiremock should record the request");
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body)?;
    assert_eq!(body["model"], "actual-upstream-model");
    assert_eq!(
        requests[0]
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer test-only-key")
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("test-only-key"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compose_routes_a_selected_shim_model_only_through_the_loopback_runtime() -> Result<()> {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(concat!(
                    "event: response.created\n",
                    "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-compose-e2e\"}}\n\n",
                    "event: response.completed\n",
                    "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-compose-e2e\",\"usage\":{\"input_tokens\":0,\"input_tokens_details\":null,\"output_tokens\":0,\"output_tokens_details\":null,\"total_tokens\":0}}}\n\n",
                )),
        )
        .expect(1)
        .mount(&upstream)
        .await;

    let workspace = TempDir::new()?;
    let codex_home = workspace.path().join("codex-home");
    std::fs::create_dir_all(&codex_home)?;
    let settings_path = workspace.path().join("models.json");
    std::fs::write(
        &settings_path,
        format!(
            r#"{{
                "providers": [{{
                    "id": "upstream",
                    "protocol": "openai-responses",
                    "base_url": "{}/v1",
                    "api_key": "compose-only-key"
                }}],
                "models": [{{
                    "slug": "compose-e2e-model",
                    "upstream_model": "actual-compose-model",
                    "provider": "upstream"
                }}]
            }}"#,
            upstream.uri()
        ),
    )?;

    let binary = codex_utils_cargo_bin::cargo_bin("codex")?;
    let output = tokio::task::spawn_blocking(move || {
        assert_cmd::Command::new(binary)
            .env("CODEX_HOME", codex_home)
            .args(["shim", "--compose", "--settings"])
            .arg(&settings_path)
            .args([
                "--",
                "-c",
                "model=\"compose-e2e-model\"",
                "exec",
                "--skip-git-repo-check",
                "--experimental-json",
                "return the composed fixture response",
            ])
            .output()
    })
    .await??;

    assert!(
        output.status.success(),
        "composed shim e2e failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = upstream
        .received_requests()
        .await
        .expect("wiremock should record the request");
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body)?;
    assert_eq!(body["model"], "actual-compose-model");
    assert_eq!(
        requests[0]
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer compose-only-key")
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("compose-only-key"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compose_auth_boundary_forbids_native_model_from_contacting_shim_upstream() -> Result<()> {
    let native_upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(concat!(
                    "event: response.created\n",
                    "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-native-compose\"}}\n\n",
                    "event: response.completed\n",
                    "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-native-compose\",\"usage\":{\"input_tokens\":0,\"input_tokens_details\":null,\"output_tokens\":0,\"output_tokens_details\":null,\"total_tokens\":0}}}\n\n",
                )),
        )
        .mount(&native_upstream)
        .await;
    let shim_upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&shim_upstream)
        .await;

    let workspace = TempDir::new()?;
    let codex_home = workspace.path().join("codex-home");
    std::fs::create_dir_all(&codex_home)?;
    let settings_path = workspace.path().join("models.json");
    std::fs::write(
        &settings_path,
        format!(
            r#"{{
                "providers": [{{
                    "id": "shim-upstream",
                    "protocol": "openai-responses",
                    "base_url": "{}/v1",
                    "api_key": "shim-only-key"
                }}],
                "models": [{{
                    "slug": "not-selected-shim-model",
                    "upstream_model": "never-selected",
                    "provider": "shim-upstream"
                }}]
            }}"#,
            shim_upstream.uri()
        ),
    )?;

    let binary = codex_utils_cargo_bin::cargo_bin("codex")?;
    let native_base_url = format!("{}/v1", native_upstream.uri());
    let output = tokio::task::spawn_blocking(move || {
        assert_cmd::Command::new(binary)
            .env("CODEX_HOME", codex_home)
            .env("OPENAI_API_KEY", "native-only-key")
            .args(["shim", "--compose", "--settings"])
            .arg(&settings_path)
            .args([
                "--",
                "-c",
                "model=\"gpt-5.6-sol\"",
                "-c",
                &format!("openai_base_url=\"{native_base_url}\""),
                "exec",
                "--skip-git-repo-check",
                "--experimental-json",
                "return the native fixture response",
            ])
            .output()
    })
    .await??;

    assert!(
        output.status.success(),
        "native composed e2e failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = native_upstream
        .received_requests()
        .await
        .expect("native upstream should record the request");
    assert!(!requests.is_empty());
    Ok(())
}
