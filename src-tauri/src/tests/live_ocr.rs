use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use foundry_local_sdk::{FoundryLocalConfig, FoundryLocalManager};

use crate::{
    check_foundry_ocr_model, commodity_extraction_schema, ensure_foundry_model_accepts_images,
    prepare_foundry_execution_providers_for_live_run, prepare_right_panel_ocr_images_in,
    run_foundry_ocr_image, select_foundry_model_variant, validate_foundry_ocr_json,
    verify_foundry_service_health, AppError, AppResult, CUDA_VISION_MODEL_ALIAS,
};

use super::fixtures;

/// Serializes live OCR tests so only one runs at a time.
static LIVE_OCR_MUTEX: Mutex<()> = Mutex::new(());

/// Milliseconds the last inference took (set by the shared thread).
static LAST_INFERENCE_MS: AtomicU64 = AtomicU64::new(0);

#[test]
fn ocr_screenshot_chieng_dark() {
    let _guard = LIVE_OCR_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    run_and_validate_fixture(&fixtures::ALL_SCREENSHOT_FIXTURES[0]);
}

#[test]
fn ocr_screenshot_arc_l1() {
    let _guard = LIVE_OCR_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    run_and_validate_fixture(&fixtures::ALL_SCREENSHOT_FIXTURES[1]);
}

#[test]
fn ocr_screenshot_hickes() {
    let _guard = LIVE_OCR_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    run_and_validate_fixture(&fixtures::ALL_SCREENSHOT_FIXTURES[2]);
}

#[test]
fn ocr_screenshot_jackson() {
    let _guard = LIVE_OCR_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    run_and_validate_fixture(&fixtures::ALL_SCREENSHOT_FIXTURES[3]);
}

#[test]
fn ocr_screenshot_theta() {
    let _guard = LIVE_OCR_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    run_and_validate_fixture(&fixtures::ALL_SCREENSHOT_FIXTURES[4]);
}

#[test]
fn live_foundry_ocr_bundled_extracts_at_least_one_commodity() {
    let _guard = LIVE_OCR_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    // Uses the first bundled screenshot instead of requiring an env var.
    let source = asset_path(fixtures::ALL_SCREENSHOT_FIXTURES[0].filename);
    let output_dir = std::env::temp_dir().join("uex-datarunner-live-ocr");
    let ocr_images = prepare_right_panel_ocr_images_in(
        &source,
        &output_dir,
        "live-ocr",
        CUDA_VISION_MODEL_ALIAS,
    )
    .unwrap_or_else(|error| panic!("live screenshot should be prepared for OCR: {error}"));

    let text = run_foundry_ocr_with_timeout(&ocr_images)
        .unwrap_or_else(|error| panic!("Foundry OCR should process live image: {error}"));
    let _ = std::fs::remove_dir_all(output_dir);

    let inference_ms = LAST_INFERENCE_MS.load(Ordering::SeqCst);
    eprintln!(
        "[perf] live_bundled: inference={:.1}s",
        inference_ms as f64 / 1000.0
    );

    validate_foundry_ocr_json(&text).expect("live OCR should return structured JSON");
    let value: serde_json::Value =
        serde_json::from_str(&text).expect("validated live OCR JSON should parse");
    assert!(
        value
            .get("commodities")
            .and_then(serde_json::Value::as_array)
            .map(|commodities| !commodities.is_empty())
            .unwrap_or(false),
        "live OCR should extract at least one commodity row. Raw OCR JSON:\n{text}"
    );
}

#[test]
fn live_ocr_step_model_alias_resolves_to_a_gpu_variant() {
    let model_alias = std::env::var("UEX_DATARUNNER_LIVE_OCR_MODEL")
        .unwrap_or_else(|_| CUDA_VISION_MODEL_ALIAS.to_string());

    let variant_id = tauri::async_runtime::block_on(async {
        tokio::time::timeout(
            Duration::from_secs(120),
            check_foundry_ocr_model(&model_alias),
        )
        .await
        .map_err(|_| AppError::Message("Model alias resolution timed out after 120s".to_string()))?
    })
    .unwrap_or_else(|error| panic!("model alias should resolve against the catalog: {error}"));

    let lowered = variant_id.to_lowercase();
    assert!(
        lowered.contains("gpu") && !lowered.contains("cpu"),
        "OCR must resolve to a GPU variant, resolved to: {variant_id}"
    );
}

#[test]
fn live_ocr_step_bundled_screenshot_returns_schema_shaped_json_without_reasoning() {
    let _guard = LIVE_OCR_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let source = asset_path("screenshot-eng-hickes.png");
    let output_dir = std::env::temp_dir().join("uex-datarunner-live-ocr-shape");
    let _ = std::fs::remove_dir_all(&output_dir);

    let images = prepare_right_panel_ocr_images_in(
        &source,
        &output_dir,
        "live-shape",
        CUDA_VISION_MODEL_ALIAS,
    )
    .unwrap_or_else(|error| panic!("screenshot should be prepared for OCR: {error}"));
    let text = run_foundry_ocr_with_timeout(&images)
        .unwrap_or_else(|error| panic!("Foundry OCR should return structured JSON: {error}"));
    let _ = std::fs::remove_dir_all(&output_dir);

    let inference_ms = LAST_INFERENCE_MS.load(Ordering::SeqCst);
    eprintln!(
        "[perf] live_shape: inference={:.1}s",
        inference_ms as f64 / 1000.0
    );

    // Uses the schema directly without requiring live server runtime logic.
    let schema = commodity_extraction_schema();
    let item_schema = schema
        .get("properties")
        .and_then(|p| p.get("commodities"))
        .and_then(|c| c.get("items"))
        .expect("schema should describe commodities items");
    let required_keys = item_schema
        .get("required")
        .and_then(serde_json::Value::as_array)
        .expect("schema should require row fields");

    validate_foundry_ocr_json(&text).expect("live OCR output should be structurally valid");
    let value: serde_json::Value =
        serde_json::from_str(&text).expect("validated live OCR JSON should parse");
    let commodities = value
        .get("commodities")
        .and_then(serde_json::Value::as_array)
        .expect("live OCR output should contain commodities array");

    assert!(
        !commodities.is_empty(),
        "live OCR must yield at least one commodity"
    );
    for row in commodities {
        for key in required_keys {
            let key = key.as_str().unwrap();
            assert!(
                row.get(key).is_some(),
                "each commodity row must include the {key} field. Raw output:\n{text}"
            );
        }
    }
}

#[test]
fn live_foundry_health_check_verifies_web_service_is_reachable() {
    let _guard = LIVE_OCR_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let model_alias = std::env::var("UEX_DATARUNNER_LIVE_OCR_MODEL")
        .unwrap_or_else(|_| CUDA_VISION_MODEL_ALIAS.to_string());

    let result = tauri::async_runtime::block_on(async {
        tokio::time::timeout(Duration::from_secs(120), async {
            let manager = foundry_local_sdk::FoundryLocalManager::create(
                foundry_local_sdk::FoundryLocalConfig::new("uex-datarunner"),
            )?;
            prepare_foundry_execution_providers_for_live_run(manager, &model_alias).await?;
            let model = manager.catalog().get_model(&model_alias).await?;
            select_foundry_model_variant(&model, &model_alias)?;
            ensure_foundry_model_accepts_images(&model)?;
            if !model.is_cached().await? {
                model.download::<fn(f64)>(None).await?;
            }
            model.load().await?;
            manager.start_web_service().await?;
            let urls = manager.urls()?;
            let base_url = urls.first().cloned().ok_or_else(|| {
                AppError::Message("Foundry web service returned no URLs.".to_string())
            })?;

            let health = verify_foundry_service_health(&base_url).await;

            let _ = manager.stop_web_service().await;
            let _ = model.unload().await;
            health
        })
        .await
        .map_err(|_| AppError::Message("Health check test timed out after 60s".to_string()))?
    });

    result.expect("health check should pass against a running Foundry web service");
}

// ---------------------------------------------------------------------------
// Shared Foundry test session
// ---------------------------------------------------------------------------

struct FoundryOcrRequest {
    image_paths: Vec<PathBuf>,
}

static FOUNDRY_THREAD: OnceLock<
    mpsc::Sender<(FoundryOcrRequest, mpsc::Sender<AppResult<String>>)>,
> = OnceLock::new();

fn foundry_thread_sender() -> mpsc::Sender<(FoundryOcrRequest, mpsc::Sender<AppResult<String>>)> {
    FOUNDRY_THREAD
        .get_or_init(|| {
            let (tx, rx) = mpsc::channel::<(FoundryOcrRequest, mpsc::Sender<AppResult<String>>)>();

            thread::spawn(move || {
                let model_alias = std::env::var("UEX_DATARUNNER_LIVE_OCR_MODEL")
                    .unwrap_or_else(|_| CUDA_VISION_MODEL_ALIAS.to_string());
                let init_result = tauri::async_runtime::block_on(async {
                    tokio::time::timeout(Duration::from_secs(120), async {
                        let manager =
                            FoundryLocalManager::create(FoundryLocalConfig::new("uex-datarunner"))?;
                        prepare_foundry_execution_providers_for_live_run(manager, &model_alias)
                            .await?;
                        let model = manager.catalog().get_model(&model_alias).await?;
                        select_foundry_model_variant(&model, &model_alias)?;
                        ensure_foundry_model_accepts_images(&model)?;
                        if !model.is_cached().await? {
                            model.download::<fn(f64)>(None).await?;
                        }
                        let _ = model.unload().await;
                        model.load().await?;
                        manager.start_web_service().await?;
                        let urls = manager.urls()?;
                        let base_url = urls.first().cloned().ok_or_else(|| {
                            AppError::Message("Foundry web service returned no URLs.".to_string())
                        })?;
                        Ok::<_, AppError>((manager, model, base_url))
                    })
                    .await
                    .map_err(|_| {
                        AppError::Message("Foundry initialization timed out after 60s".to_string())
                    })?
                });

                match init_result {
                    Ok((manager, model, base_url)) => {
                        while let Ok((req, respond_to)) = rx.recv() {
                            let inference_start = Instant::now();
                            let ready_result = tauri::async_runtime::block_on(async {
                                if !model.is_loaded().await? {
                                    model.load().await?;
                                }
                                Ok::<_, AppError>(())
                            });

                            let result = match ready_result {
                                Ok(()) if req.image_paths.len() == 1 => {
                                    tauri::async_runtime::block_on(async {
                                        tokio::time::timeout(
                                            Duration::from_secs(120),
                                            run_foundry_ocr_image(
                                                None,
                                                &base_url,
                                                &model,
                                                &req.image_paths[0],
                                            ),
                                        )
                                        .await
                                        .map_err(|_| {
                                            AppError::Message(
                                                "OCR request timed out inside thread after 60s"
                                                    .to_string(),
                                            )
                                        })?
                                    })
                                }
                                Ok(_) => Err(AppError::Message(
                                    "Multi-image OCR not supported in test thread".to_string(),
                                )),
                                Err(error) => Err(error),
                            };

                            let elapsed_ms = inference_start.elapsed().as_millis() as u64;
                            LAST_INFERENCE_MS.store(elapsed_ms, Ordering::SeqCst);
                            let _ = respond_to.send(result);
                        }
                        tauri::async_runtime::block_on(async {
                            let _ = manager.stop_web_service().await;
                            let _ = model.unload().await;
                        });
                    }
                    Err(e) => {
                        while let Ok((_, respond_to)) = rx.recv() {
                            let _ = respond_to.send(Err(AppError::Message(format!(
                                "Foundry initialization failed: {e}"
                            ))));
                        }
                    }
                }
            });

            tx
        })
        .clone()
}

/// Runs OCR through the shared Foundry thread with a timeout.
/// The timeout must exceed the blocking HTTP client timeout, otherwise the
/// test can fail while the worker keeps running and contaminates later OCR
/// cases.
fn run_foundry_ocr_with_timeout(image_paths: &[PathBuf]) -> AppResult<String> {
    let sender = foundry_thread_sender();
    let (respond_tx, respond_rx) = mpsc::channel();
    sender
        .send((
            FoundryOcrRequest {
                image_paths: image_paths.to_vec(),
            },
            respond_tx,
        ))
        .map_err(|e| AppError::Message(format!("Foundry thread is dead: {e}")))?;
    let timeout = Duration::from_secs(120);
    match respond_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => Err(AppError::Message(format!(
            "OCR timed out after {}s",
            timeout.as_secs()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Per-screenshot OCR test helpers
// ---------------------------------------------------------------------------

fn run_and_validate_fixture(fixture: &fixtures::ScreenshotFixture) {
    let path = asset_path(fixture.filename);
    assert!(
        path.exists(),
        "Expected OCR fixture screenshot to exist: {}",
        path.display()
    );

    let output_dir = std::env::temp_dir().join(format!(
        "uex-datarunner-ocr-{}",
        fixture.filename.replace('.', "-")
    ));
    let ocr_images = prepare_right_panel_ocr_images_in(
        &path,
        &output_dir,
        fixture.filename,
        CUDA_VISION_MODEL_ALIAS,
    )
    .unwrap_or_else(|error| panic!("{} should be prepared for OCR: {}", fixture.filename, error));

    let text = run_foundry_ocr_with_timeout(&ocr_images).unwrap_or_else(|error| {
        panic!("Foundry OCR should process {}: {}", fixture.filename, error)
    });
    let _ = std::fs::remove_dir_all(&output_dir);

    let inference_ms = LAST_INFERENCE_MS.load(Ordering::SeqCst);
    eprintln!(
        "[perf] {}: inference={:.1}s",
        fixture.filename,
        inference_ms as f64 / 1000.0
    );

    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!(
            "OCR output for {} should be valid JSON: {}\nRaw: {}",
            fixture.filename, error, text
        )
    });

    let market_side = value.get("marketSide").and_then(|v| v.as_str());
    assert_eq!(
        market_side,
        Some(fixture.market_side),
        "OCR for {} should have marketSide={}, got {:?}. Raw: {}",
        fixture.filename,
        fixture.market_side,
        market_side,
        text
    );

    let commodities = value
        .get("commodities")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("OCR for {} should have commodities array", fixture.filename));

    for expected in fixture.commodities {
        let matched = commodities.iter().find(|row| {
            row.get("name")
                .and_then(|v| v.as_str())
                .map(|n| canonical_test_name(n) == canonical_test_name(expected.name))
                .unwrap_or(false)
        });

        let matched = match matched {
            Some(row) => row,
            None if expected.optional => continue,
            None => panic!(
                "OCR for {} should contain commodity '{}'. Got commodities: {:?}",
                fixture.filename, expected.name, commodities
            ),
        };

        assert_commodity_field(
            fixture.filename,
            expected.name,
            "scu",
            expected.scu,
            matched.get("scu").and_then(|v| v.as_f64()),
        );
        assert_commodity_field(
            fixture.filename,
            expected.name,
            "pricePerScu",
            expected.price_per_scu,
            matched.get("pricePerScu").and_then(|v| v.as_f64()),
        );
        let status_raw = matched.get("status").and_then(|v| v.as_i64());
        assert_commodity_field(
            fixture.filename,
            expected.name,
            "status",
            expected.status,
            status_raw,
        );

        if !expected.cargo_sizes.is_empty() {
            let actual: Vec<i64> = matched
                .get("cargoSizes")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
                .unwrap_or_default();
            let expected_vec = expected.cargo_sizes.to_vec();
            assert_eq!(
                actual, expected_vec,
                "OCR for {} commodity '{}' has wrong cargoSizes",
                fixture.filename, expected.name
            );
        }
    }
}

fn assert_commodity_field<T: std::fmt::Debug + PartialEq>(
    filename: &str,
    commodity: &str,
    field: &str,
    expected: Option<T>,
    actual: Option<T>,
) {
    if let Some(expected_value) = expected {
        assert_eq!(
            actual,
            Some(expected_value),
            "OCR for {} commodity '{}' field '{}' mismatch",
            filename,
            commodity,
            field
        );
    }
}

fn canonical_test_name(value: &str) -> String {
    let mut s: String = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    if s == "agricicum" {
        s = "agricium".to_string();
    }
    s
}

fn asset_path(filename: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tests")
        .join("asset")
        .join(filename)
}
