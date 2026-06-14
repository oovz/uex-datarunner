use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::{
    emit_ocr_batch_started, emit_ocr_finished, emit_ocr_progress, emit_ocr_screenshot_failed,
    emit_ocr_screenshot_started, emit_ocr_screenshot_succeeded, log_event, read_config, AppError,
    AppResult, AppState, ProcessScreenshotsResult, ProcessedScreenshot, ScreenshotFile,
};
use base64::Engine;
use foundry_local_sdk::{FoundryLocalConfig, FoundryLocalManager};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub(crate) const FOUNDRY_OCR_HTTP_TIMEOUT_SECS: u64 = 60;
pub(crate) const FOUNDRY_OCR_MAX_TOKENS: u32 = 4096;
pub(crate) const CUDA_VISION_MODEL_ALIAS: &str = "qwen3-vl-4b-instruct";
pub(crate) const QWEN3_VL_4B_INSTRUCT_CUDA_ALIAS: &str = "qwen3-vl-4b-instruct";
pub(crate) const OCR_IMAGE_MAX_SIDE: f64 = 960.0;

const FOUNDRY_OCR_INSTRUCTIONS: &str = r#"Extract commodity data from terminal screenshots. Your response must consist of a plain text transcription list of each commodity's name, its status label text, and its mapped status code (1 to 7), followed by a raw JSON object starting with '{' and ending with '}'. No markdown code blocks. Every response must include the top-level fields "marketSide", "activeTab", "listHeader" followed by the "commodities" array. Every commodity object must use exactly these keys once: name (exact, not translated), status, scu, pricePerScu, "cargoSizes".

Inside your transcription list, write down:
1. Identify all commodities listed in the table.
2. For each commodity:
   - name: transcribe the commodity name exactly.
   - label: transcribe the exact text label directly below the name (e.g. "库存已满", "库存充足", "库存中等", "库存已空", "OUT OF STOCK", etc.).
   - status: map the transcribed text label directly below the name to a status code (1 to 7) using these mappings (for English, Chinese, or other languages):
      - Out of stock / Empty / "库存已空" / "OUT OF STOCK" / "OUT" -> 1
      - Very low / "库存极低" / "VERY LOW" -> 2
      - Low / "库存偏少" / "LOW" -> 3
      - Medium / "库存中等" / "MEDIUM" -> 4
      - High / "库存充足" / "HIGH" -> 5
      - Very high / "库存将满" / "VERY HIGH" -> 6
      - Full / Maximum / "库存已满" / "FULL" -> 7
      
      CRITICAL STATUS RULES:
      - The status code mapping must be based STRICTLY on the text label, not the SCU quantity or price. For example, a quantity of 1 SCU can still be status 7 ("库存已满"), and a quantity of 1973 SCU can be status 5 ("库存充足").
      - If the text label directly below the name is "库存已满" or "FULL", the status code is ALWAYS 7. Do NOT map it to 1 or 5.
      - If the text label directly below the name is "库存将满" or "VERY HIGH", the status code is ALWAYS 6. Do NOT map it to 4 or 7.
      - Distinguish very carefully between the Chinese characters:
        * "库存将满" (Very High) -> Status 6. The character "将" (jiang) indicates "about to be full". You must transcribe this exactly and map to status 6.
        * "库存已满" (Full) -> Status 7. The character "已" (yi) indicates "already full". You must transcribe this exactly and map to status 7.
        * Do not drop or ignore the character "将" or confuse it with "已".
   - cargoSizes: carefully transcribe all numbers in the cargo size boxes. Note: when the commodity is out of stock, the boxes are grayed out and dim, but you must read them carefully. Look for 24, 32, etc. Do not miss the number 24.
   - price/quantity: transcribe the SCU quantity and price. WARNING: Ignore the currency symbol ¤. Do NOT read ¤ as digit '1' (e.g., ¤63,000 is 63000, NOT 163000).
"#;

const FOUNDRY_OCR_USER_PROMPT_TEMPLATE: &str = r#"First, transcribe the name and the exact status text label directly below the name for each commodity, and map the label to the correct status code (1 to 7) in a plain text list. 

Example:
- name: CommodityA, label: 库存充足, status: 5
- name: CommodityB, label: 库存已满, status: 7

After the list, Return a raw JSON object matching this schema template:
{
  "activeTab": "transcribe the exact text of the active/selected main tab (e.g. Buy/购买, Local Market Value/本地市场价格, Sell/出售). Look at which tab is highlighted/selected.",
  "listHeader": "transcribe the topmost header or section title at the very top of the panel (e.g. SHOP INVENTORY, SELLABLE CARGO, IN STOCK, IN DEMAND, 有货, 有需求, 货物清单, 售罄). Do NOT transcribe section titles from the bottom like NO DEMAND.",
  "marketSide": "buy if activeTab is 'Buy' / '购买'; sell if activeTab is 'Local Market Value' / '本地市场价格' or 'Sell' / '出售'.",
  "commodities": [
    {
      "name": "exact spelling of commodity name from screenshot, do not translate (e.g. CommodityName)",
      "status": 7,
      "scu": 1.0,
      "pricePerScu": "number representing price per SCU. WARNING: The currency symbol ¤ is displayed before price numbers (e.g. ¤63,000, ¤68,000). Do NOT read the ¤ symbol as digit '1'. Ignore ¤ and extract only actual digits, e.g. for ¤63,000 write 63000 (NOT 163000). Do not prepend leading '1' to any price.",
      "cargoSizes": [1, 2]
    }
  ]
}

Please map "status" and extract "cargoSizes" strictly following the system instructions. Do not include markdown code block formatting in your final response."#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FoundryVariantChoice {
    pub(crate) id: String,
    pub(crate) provider_type: String,
    pub(crate) execution_provider: Option<String>,
    pub(crate) device_type: Option<String>,
    pub(crate) input_modalities: String,
    pub(crate) cached: bool,
}

#[cfg(test)]
impl FoundryVariantChoice {
    pub(crate) fn for_test(
        id: &str,
        provider_type: &str,
        execution_provider: Option<&str>,
        device_type: Option<&str>,
        input_modalities: &str,
        cached: bool,
    ) -> Self {
        Self {
            id: id.to_string(),
            provider_type: provider_type.to_string(),
            execution_provider: execution_provider.map(|s| s.to_string()),
            device_type: device_type.map(|s| s.to_string()),
            input_modalities: input_modalities.to_string(),
            cached,
        }
    }
}

mod client;
mod hardware;
mod image;
mod json;
mod model;

#[cfg(test)]
pub(crate) use hardware::default_ai_model;
#[cfg(test)]
pub(crate) use hardware::find_cuda_runtime_dir_in_roots;
pub(crate) use hardware::{
    configured_ai_model, get_system_gpu_info, is_cuda_runtime_available,
    validate_ocr_hardware_policy,
};

#[cfg(test)]
pub(crate) use client::is_foundry_transport_failure;
#[cfg(test)]
#[cfg(test)]
pub(crate) use client::{run_foundry_ocr_image, verify_foundry_service_health};
#[cfg(test)]
pub(crate) use image::prepare_right_panel_ocr_images_in;
#[cfg(test)]
pub(crate) use json::{
    commodity_extraction_schema, foundry_ocr_messages, merge_foundry_ocr_results,
    normalize_foundry_ocr_json, strip_model_reasoning, validate_foundry_ocr_json,
};
#[cfg(test)]
pub(crate) use model::{
    prepare_foundry_execution_providers_for_live_run, select_foundry_model_variant,
    select_foundry_vision_variant,
};

pub(crate) async fn process_files_with_foundry_ocr(
    app: &AppHandle,
    files: &[ScreenshotFile],
    model_alias: &str,
) -> AppResult<ProcessScreenshotsResult> {
    let mut screenshots = Vec::new();
    let mut warnings = Vec::new();
    let keep_foundry_session_loaded = read_config(app)?.keep_model_loaded;

    if files.is_empty() {
        return Ok(ProcessScreenshotsResult {
            screenshots,
            warnings,
        });
    }

    emit_ocr_progress(app, "Creating Foundry Local manager...");
    let manager_start = Instant::now();
    let manager = FoundryLocalManager::create(FoundryLocalConfig::new("uex-datarunner"))?;
    emit_ocr_progress(
        app,
        &format!(
            "Foundry Local manager created in {:.2}s",
            manager_start.elapsed().as_secs_f64()
        ),
    );
    model::prepare_foundry_execution_providers(app, manager, model_alias).await?;
    emit_ocr_progress(
        app,
        &format!("Fetching model '{}' from catalog...", model_alias),
    );
    let catalog_start = Instant::now();
    let model = manager.catalog().get_model(model_alias).await?;
    emit_ocr_progress(
        app,
        &format!(
            "Model catalog lookup completed in {:.2}s",
            catalog_start.elapsed().as_secs_f64()
        ),
    );
    model::select_foundry_model_variant(&model, model_alias)?;
    emit_ocr_progress(app, &format!("Model resolved to variant: '{}'", model.id()));
    log_event("OCR", format!("Model '{}' variants:", model_alias));
    for variant in model.variants() {
        let info = variant.info();
        let runtime = info
            .runtime
            .as_ref()
            .map(|runtime| format!("{:?}/{}", runtime.device_type, runtime.execution_provider))
            .unwrap_or_else(|| "unknown runtime".to_string());
        log_event(
            "OCR",
            format!(
                "  Variant: {} (provider={}, runtime={}, cached={})",
                variant.id(),
                info.provider_type,
                runtime,
                variant.is_cached().await.unwrap_or(false)
            ),
        );
    }

    let _loaded_model_id = model::load_foundry_model_for_gpu_only(app, &model, model_alias).await?;
    let base_url = match manager.urls()?.first().cloned() {
        Some(url) => {
            emit_ocr_progress(app, "Reusing running Foundry Local web service");
            url
        }
        None => {
            emit_ocr_progress(app, "Starting Foundry Local web service...");
            let service_start = Instant::now();
            manager.start_web_service().await?;
            let urls = manager.urls()?;
            let url = urls.first().cloned().ok_or_else(|| {
                AppError::Message("Foundry web service returned no URLs.".to_string())
            })?;
            emit_ocr_progress(
                app,
                &format!(
                    "Web service started in {:.2}s",
                    service_start.elapsed().as_secs_f64()
                ),
            );
            url
        }
    };
    log_event(
        "OCR",
        format!(
            "Web service URL: {}, model variant: '{}'",
            base_url,
            model.id()
        ),
    );

    // Verify the web service is actually responding before sending inference
    if let Err(health_err) = client::verify_foundry_service_health(&base_url).await {
        emit_ocr_progress(
            app,
            &format!("Web service health check failed: {health_err}."),
        );
        log_event("OCR", format!("Health check failed: {health_err}"));
        return Err(AppError::Message(format!(
            "Foundry web service failed health check: {health_err}"
        )));
    }

    emit_ocr_progress(app, "Web service ready. Processing screenshots...");
    emit_ocr_batch_started(app, files.len());

    for (index, file) in files.iter().enumerate() {
        if app
            .state::<AppState>()
            .is_ocr_cancelled
            .load(Ordering::Relaxed)
        {
            emit_ocr_progress(app, "OCR cancelled by user.");
            break;
        }

        emit_ocr_progress(
            app,
            &format!(
                "Processing {} ({}/{})",
                file.filename,
                index + 1,
                files.len()
            ),
        );
        emit_ocr_screenshot_started(app, file, index + 1, files.len());
        let stem = Path::new(&file.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("ocr-crop");
        match process_single_screenshot_async(app, file, &base_url, &model, stem).await {
            Ok(processed) => {
                emit_ocr_progress(
                    app,
                    &format!(
                        "Done {} - extracted {} characters",
                        file.filename,
                        processed.ocr_text.len()
                    ),
                );
                emit_ocr_screenshot_succeeded(app, file);
                screenshots.push(processed);
            }
            Err(error) => {
                emit_ocr_progress(app, &format!("Failed {}: {}", file.filename, error));
                emit_ocr_screenshot_failed(app, file, &error.to_string());
                warnings.push(format!("{}: {}", file.filename, error));
            }
        }
    }

    if keep_foundry_session_loaded {
        emit_ocr_progress(app, "Keeping Foundry Local model and web service loaded");
    } else {
        emit_ocr_progress(app, "Stopping web service...");
        let _ = manager.stop_web_service().await;
        emit_ocr_progress(app, "Unloading model...");
        let _ = model.unload().await;
    }
    emit_ocr_progress(
        app,
        &format!(
            "Finished. {} processed, {} warnings",
            screenshots.len(),
            warnings.len()
        ),
    );
    emit_ocr_finished(app, screenshots.len(), warnings.len());

    Ok(ProcessScreenshotsResult {
        screenshots,
        warnings,
    })
}

pub(crate) async fn process_single_screenshot_async(
    app: &AppHandle,
    file: &ScreenshotFile,
    base_url: &str,
    model: &foundry_local_sdk::Model,
    stem: &str,
) -> AppResult<ProcessedScreenshot> {
    let start = std::time::Instant::now();
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| AppError::Message(error.to_string()))?;
    std::fs::create_dir_all(&cache_dir)?;
    let crop_paths = image::prepare_right_panel_ocr_images_in(
        Path::new(&file.path),
        &cache_dir,
        stem,
        model.id(),
    )?;
    let ocr_result =
        client::run_foundry_ocr_requests(Some(app), base_url, model, &crop_paths).await;
    for crop_path in &crop_paths {
        let _ = std::fs::remove_file(crop_path);
    }
    let ocr_text = ocr_result?;
    let bytes = std::fs::read(&file.path)?;
    let screenshot_base64 = base64::engine::general_purpose::STANDARD.encode(bytes);

    let elapsed = start.elapsed();
    log_event(
        "ocr",
        format!(
            "Total for {}: {:.2}s ({} chars)",
            file.filename,
            elapsed.as_secs_f64(),
            ocr_text.len()
        ),
    );

    Ok(ProcessedScreenshot {
        file: file.clone(),
        ocr_text,
        screenshot_base64,
    })
}

pub(crate) async fn check_foundry_ocr_model(model_alias: &str) -> AppResult<String> {
    log_event(
        "OCR",
        format!("Checking Foundry Local OCR model: {}", model_alias),
    );
    let manager = FoundryLocalManager::create(FoundryLocalConfig::new("uex-datarunner"))?;
    let model = manager.catalog().get_model(model_alias).await?;
    let selected = model::select_foundry_model_variant(&model, model_alias)?;
    let cached = model.is_cached().await?;
    log_event(
        "OCR",
        format!(
            "Model '{}' selected '{}' (cached={})",
            model_alias, selected.id, cached
        ),
    );
    Ok(selected.id)
}

pub(crate) async fn foundry_loaded_model_status(
    model_alias: &str,
) -> AppResult<(bool, Option<String>)> {
    let manager = FoundryLocalManager::create(FoundryLocalConfig::new("uex-datarunner"))?;
    let loaded = manager.catalog().get_loaded_models().await?;
    let loaded_id = loaded
        .iter()
        .map(|model| model.id().to_string())
        .find(|id| id.starts_with(model_alias));
    Ok((loaded_id.is_some(), loaded_id))
}

pub(crate) fn ensure_foundry_model_accepts_images(
    model: &foundry_local_sdk::Model,
) -> AppResult<()> {
    let modalities = model.input_modalities().unwrap_or("");
    if !modalities
        .split(',')
        .map(str::trim)
        .any(|modality| modality.eq_ignore_ascii_case("image"))
    {
        return Err(AppError::Message(format!(
            "Foundry model '{}' does not advertise image input modalities: '{}'",
            model.id(),
            modalities
        )));
    }

    Ok(())
}
