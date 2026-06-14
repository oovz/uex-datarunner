use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::{emit_ocr_progress, log_event, AppError, AppResult, AppState};
use foundry_local_sdk::FoundryLocalManager;
use serde_json::Value;
use tauri::{AppHandle, Manager};

// Retry constants removed. We do not retry after an image failed.

/// Timeout for the lightweight health-check probe.
const FOUNDRY_HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) async fn run_foundry_ocr_requests(
    app: Option<&AppHandle>,
    base_url: &str,
    model: &foundry_local_sdk::Model,
    image_paths: &[PathBuf],
) -> AppResult<String> {
    let mut results = Vec::new();
    for image_path in image_paths {
        if let Some(app) = app {
            if app
                .state::<AppState>()
                .is_ocr_cancelled
                .load(Ordering::Relaxed)
            {
                emit_ocr_progress(app, "OCR cancelled by user.");
                return Err(AppError::Message("OCR cancelled by user.".to_string()));
            }
        }
        let text = run_foundry_ocr_image(app, base_url, model, image_path).await?;
        results.push(text);
    }

    super::json::merge_foundry_ocr_results(&results)
}

pub(crate) async fn run_foundry_ocr_image(
    app: Option<&AppHandle>,
    _base_url: &str,
    _model: &foundry_local_sdk::Model,
    image_path: &std::path::Path,
) -> AppResult<String> {
    let image_base64 = super::image::foundry_image_data(image_path, _model.id())?;
    let request_body = super::json::foundry_ocr_messages(_model.id(), &image_base64)?;
    let model_id = request_body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("");

    let body_bytes = serde_json::to_vec(&request_body)
        .map(|v| v.len())
        .unwrap_or(0);
    log_event(
        "OCR",
        format!(
            "Inference request: model={model_id}, endpoint={_base_url}/v1/responses, \
             image_base64_len={}, body_size={body_bytes} bytes",
            image_base64.len()
        ),
    );
    log_event("ocr", format!("responses request (model={model_id})"));

    let start = Instant::now();
    match send_foundry_ocr_request(app, _base_url, &request_body).await {
        Ok(response_text) => Ok(response_text),
        Err(err) => {
            let elapsed = start.elapsed();
            let is_transport = is_foundry_transport_failure(&err);
            log_event(
                "OCR",
                format!(
                    "OCR request failed (elapsed={:.2}s, transport_failure={}): {}",
                    elapsed.as_secs_f64(),
                    is_transport,
                    err
                ),
            );
            Err(err)
        }
    }
}

/// Sends a single inference request to the Foundry web service and returns the
/// normalized OCR JSON text on success.
async fn send_foundry_ocr_request(
    app: Option<&AppHandle>,
    base_url: &str,
    request_body: &Value,
) -> AppResult<String> {
    let start = Instant::now();

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(super::FOUNDRY_OCR_HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Message(format!("Failed to build HTTP client: {e}")))?;
    let request = http_client
        .post(format!("{base_url}/v1/responses"))
        .json(request_body)
        .send();
    let http_response = if let Some(app) = app {
        tokio::select! {
            response = request => response.map_err(foundry_http_send_error)?,
            _ = wait_for_ocr_cancel(app) => {
                return Err(AppError::Message("OCR cancelled by user.".to_string()));
            }
        }
    } else {
        request.await.map_err(foundry_http_send_error)?
    };

    let status = http_response.status();
    log_event(
        "ocr",
        format!(
            "HTTP response: status={status}, elapsed={:.2}s",
            start.elapsed().as_secs_f64()
        ),
    );

    if !status.is_success() {
        let body = http_response.text().await.unwrap_or_default();
        log_event(
            "OCR",
            format!(
                "Foundry HTTP error: status={status}, body_len={}, body={}",
                body.len(),
                body.chars().take(500).collect::<String>()
            ),
        );
        return Err(AppError::Message(format!(
            "Foundry OCR HTTP error {status}: {body}"
        )));
    }

    let response: Value = http_response.json().await?;
    let elapsed_secs = start.elapsed().as_secs_f64();
    let raw_text = super::json::foundry_ocr_response_text(&response)?;
    log_event(
        "ocr",
        format!("Raw response ({} chars): {}", raw_text.len(), raw_text),
    );
    let text = super::json::normalize_foundry_ocr_json(&raw_text)?;
    log_event("ocr", format!("Response received in {elapsed_secs:.2}s"));
    log_event(
        "ocr",
        format!("Extracted JSON ({} chars): {}", text.len(), text),
    );
    Ok(text)
}

/// Performs a lightweight health check against the Foundry Local web service by
/// querying the `/v1/models` endpoint. This confirms the HTTP server is
/// listening and can serve requests before we send the expensive inference
/// payload.
pub(crate) async fn verify_foundry_service_health(base_url: &str) -> AppResult<()> {
    let url = format!("{base_url}/v1/models");
    log_event("OCR", format!("Health check: GET {url}"));
    let start = Instant::now();

    let client = reqwest::Client::builder()
        .timeout(FOUNDRY_HEALTH_CHECK_TIMEOUT)
        .build()
        .map_err(|e| AppError::Message(format!("Failed to build health-check client: {e}")))?;

    match client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let elapsed = start.elapsed();
            if status.is_success() {
                log_event(
                    "OCR",
                    format!(
                        "Health check passed: status={status}, elapsed={:.2}s",
                        elapsed.as_secs_f64()
                    ),
                );
                Ok(())
            } else {
                let body = resp.text().await.unwrap_or_default();
                log_event(
                    "OCR",
                    format!(
                        "Health check failed: status={status}, body={}",
                        body.chars().take(200).collect::<String>()
                    ),
                );
                Err(AppError::Message(format!(
                    "Foundry health check failed: HTTP {status}"
                )))
            }
        }
        Err(e) => {
            let elapsed = start.elapsed();
            log_event(
                "OCR",
                format!(
                    "Health check error: elapsed={:.2}s, error={e}",
                    elapsed.as_secs_f64()
                ),
            );
            Err(AppError::Message(format!(
                "Foundry web service is not responding: {e}"
            )))
        }
    }
}

async fn wait_for_ocr_cancel(app: &AppHandle) {
    loop {
        if app
            .state::<AppState>()
            .is_ocr_cancelled
            .load(Ordering::Relaxed)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[allow(dead_code)]
pub(crate) fn run_foundry_ocr_image_blocking(
    base_url: &str,
    model: &foundry_local_sdk::Model,
    image_path: &std::path::Path,
) -> AppResult<String> {
    let image_base64 = super::image::foundry_image_data(image_path, model.id())?;
    let request_body = super::json::foundry_ocr_messages(model.id(), &image_base64)?;
    let model_id = request_body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("");
    log_event("ocr", format!("responses request (model={model_id})"));
    let start = Instant::now();

    let http_client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(super::FOUNDRY_OCR_HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Message(format!("Failed to build HTTP client: {e}")))?;
    let http_response = http_client
        .post(format!("{base_url}/v1/responses"))
        .json(&request_body)
        .send()
        .map_err(foundry_http_send_error)?;

    if !http_response.status().is_success() {
        let status = http_response.status();
        let body = http_response.text().unwrap_or_default();
        return Err(AppError::Message(format!(
            "Foundry OCR HTTP error {status}: {body}"
        )));
    }

    let response: Value = http_response
        .json()
        .map_err(|e| AppError::Message(format!("Failed to parse JSON response: {e}")))?;
    let elapsed_secs = start.elapsed().as_secs_f64();
    let raw_text = super::json::foundry_ocr_response_text(&response)?;
    log_event(
        "ocr",
        format!("Raw response ({} chars): {}", raw_text.len(), raw_text),
    );
    let text = super::json::normalize_foundry_ocr_json(&raw_text)?;
    log_event("ocr", format!("Response received in {elapsed_secs:.2}s"));
    log_event(
        "ocr",
        format!("Extracted JSON ({} chars): {}", text.len(), text),
    );
    Ok(text)
}

#[allow(dead_code)]
pub(crate) fn run_foundry_ocr_candidates(
    image_paths: &[PathBuf],
    model_alias: &str,
) -> AppResult<String> {
    tauri::async_runtime::block_on(run_foundry_ocr_candidates_async(image_paths, model_alias))
}

#[allow(dead_code)]
pub(crate) async fn run_foundry_ocr_candidates_async(
    image_paths: &[PathBuf],
    model_alias: &str,
) -> AppResult<String> {
    let manager =
        FoundryLocalManager::create(foundry_local_sdk::FoundryLocalConfig::new("uex-datarunner"))?;
    super::model::prepare_foundry_execution_providers_for_live_run(manager, model_alias).await?;
    let model = manager.catalog().get_model(model_alias).await?;
    super::model::select_foundry_model_variant(&model, model_alias)?;

    super::ensure_foundry_model_accepts_images(&model)?;
    if !model.is_cached().await? {
        model.download::<fn(f64)>(None).await?;
    }
    model.load().await?;
    manager.start_web_service().await?;
    let urls = manager.urls()?;
    let base_url = urls
        .first()
        .ok_or_else(|| AppError::Message("Foundry web service returned no URLs.".to_string()))?;

    let result = run_foundry_ocr_requests(None, base_url, &model, image_paths).await;
    let _ = manager.stop_web_service().await;
    let _ = model.unload().await;
    result
}

fn foundry_http_send_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        return AppError::Message(format!(
            "Foundry OCR request timed out after {}s. Cause: {}",
            super::FOUNDRY_OCR_HTTP_TIMEOUT_SECS,
            error
        ));
    }

    if error.is_connect() {
        return AppError::Message(format!(
            "Foundry OCR connection failed (service may not be running): {error}"
        ));
    }

    AppError::Message(format!("Foundry OCR request failed: {error}"))
}

pub(crate) fn is_foundry_transport_failure(error: &AppError) -> bool {
    let message = error.to_string();
    message.contains("Foundry OCR request timed out")
        || message.contains("Foundry OCR request failed")
        || message.contains("Foundry OCR connection failed")
        || message.contains("Foundry web service is not responding")
}
