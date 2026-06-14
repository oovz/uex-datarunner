use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::{emit_ocr_progress, log_event, AppError, AppResult, AppState};
use foundry_local_sdk::FoundryLocalManager;
use tauri::{AppHandle, Manager};

pub(crate) async fn load_foundry_model_for_gpu_only(
    app: &AppHandle,
    model: &foundry_local_sdk::Model,
    model_alias: &str,
) -> AppResult<String> {
    super::ensure_foundry_model_accepts_images(model)?;
    reject_unsupported_model_variant(model.id())?;

    if model.is_cached().await? {
        emit_ocr_progress(
            app,
            &format!("Model '{}' already cached locally", model_alias),
        );
    } else {
        emit_ocr_progress(app, &format!("Downloading model '{}'...", model_alias));
        let download_start = Instant::now();
        model.download::<fn(f64)>(None).await?;
        emit_ocr_progress(
            app,
            &format!(
                "Model '{}' downloaded in {:.2}s",
                model_alias,
                download_start.elapsed().as_secs_f64()
            ),
        );
    }

    reject_unsupported_model_variant(model.id())?;
    if model.is_loaded().await? {
        emit_ocr_progress(
            app,
            &format!(
                "Model already loaded: '{}' (variant: '{}')",
                model_alias,
                model.id()
            ),
        );
        return Ok(model.id().to_string());
    }

    // Log pre-load state for diagnostics
    let pre_cached = model.is_cached().await.unwrap_or(false);
    let pre_loaded = model.is_loaded().await.unwrap_or(false);
    log_event(
        "OCR",
        format!(
            "Pre-load state: model='{}', variant='{}', cached={}, loaded={}",
            model_alias,
            model.id(),
            pre_cached,
            pre_loaded
        ),
    );

    emit_ocr_progress(app, &format!("Loading model '{}'...", model_alias));
    let load_start = Instant::now();
    model.load().await.map_err(|error| {
        AppError::Message(format!(
            "Failed to load GPU model '{}': {}. CPU and WebGPU model variants are not supported.",
            model_alias, error
        ))
    })?;
    reject_unsupported_model_variant(model.id())?;

    let load_elapsed = load_start.elapsed();
    emit_ocr_progress(
        app,
        &format!(
            "Model loaded: '{}' (variant: '{}')",
            model_alias,
            model.id()
        ),
    );
    emit_ocr_progress(
        app,
        &format!("Model load completed in {:.2}s", load_elapsed.as_secs_f64()),
    );

    // Post-load verification: confirm the model reports as loaded
    let post_loaded = model.is_loaded().await.unwrap_or(false);
    log_event(
        "OCR",
        format!(
            "Post-load verification: model='{}', variant='{}', loaded={}, load_time={:.2}s",
            model_alias,
            model.id(),
            post_loaded,
            load_elapsed.as_secs_f64()
        ),
    );
    if !post_loaded {
        let msg = format!(
            "Model '{}' (variant '{}') reported load() success but is_loaded() returned false. \
             The Foundry Local runtime may have failed to initialize the model.",
            model_alias,
            model.id()
        );
        log_event("OCR", &msg);
        return Err(AppError::Message(msg));
    }

    Ok(model.id().to_string())
}

pub(crate) fn reject_cpu_model_variant(model_id: &str) -> AppResult<()> {
    if model_id.to_lowercase().contains("cpu") {
        return Err(AppError::Message(format!(
            "Foundry selected CPU model variant '{model_id}', but CPU inference is not supported."
        )));
    }

    Ok(())
}

pub(crate) fn reject_unsupported_model_variant(model_id: &str) -> AppResult<()> {
    reject_cpu_model_variant(model_id)?;
    if contains_any(model_id, &["webgpu", "web-gpu"]) {
        return Err(AppError::Message(format!(
            "Foundry selected WebGPU model variant '{model_id}', but this app requires NVIDIA CUDA."
        )));
    }

    Ok(())
}

pub(crate) async fn prepare_foundry_execution_providers(
    app: &AppHandle,
    manager: &FoundryLocalManager,
    model_alias: &str,
) -> AppResult<()> {
    emit_ocr_progress(app, "Checking Foundry execution providers...");
    let eps = manager.discover_eps()?;
    let target_names = select_required_execution_provider_names(model_alias, &eps)?;
    let target_names_known_in_session = app
        .state::<AppState>()
        .registered_execution_providers
        .lock()
        .map(|registered| target_names.iter().all(|name| registered.contains(name)))
        .unwrap_or(false);
    if target_names.iter().all(|name| {
        eps.iter()
            .find(|ep| ep.name == *name)
            .map(|ep| ep.is_registered)
            .unwrap_or(false)
    }) || target_names_known_in_session
    {
        emit_ocr_progress(app, "Required Foundry execution providers are registered.");
        return Ok(());
    }

    let refs: Vec<&str> = target_names.iter().map(String::as_str).collect();
    let progress_app = app.clone();
    let last_progress: std::sync::Arc<Mutex<HashMap<String, u8>>> =
        std::sync::Arc::new(Mutex::new(HashMap::new()));
    let progress_state = std::sync::Arc::clone(&last_progress);
    let register_start = Instant::now();
    let result = manager
        .download_and_register_eps_with_progress(Some(&refs), move |ep_name, percent| {
            let rounded = percent.round().clamp(0.0, 100.0) as u8;
            let should_emit = progress_state
                .lock()
                .map(|mut state| {
                    let previous = state.insert(ep_name.to_string(), rounded);
                    previous != Some(rounded)
                })
                .unwrap_or(true);
            if should_emit {
                emit_ocr_progress(
                    &progress_app,
                    &format!("Registering Foundry EP {ep_name}: {rounded}%"),
                );
            }
        })
        .await?;

    if !result.success {
        let cuda_runtime = super::hardware::find_cuda_runtime_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not detected".to_string());
        return Err(AppError::Message(format!(
            "Foundry could not register required execution providers {:?}: {}. Failed: {:?}. CUDA runtime directory: {}",
            target_names, result.status, result.failed_eps, cuda_runtime
        )));
    }

    if let Ok(mut registered) = app
        .state::<AppState>()
        .registered_execution_providers
        .lock()
    {
        registered.extend(target_names.iter().cloned());
    }
    emit_ocr_progress(
        app,
        &format!(
            "Foundry execution providers registered in {:.2}s.",
            register_start.elapsed().as_secs_f64()
        ),
    );
    Ok(())
}

pub(crate) fn select_required_execution_provider_names(
    model_alias: &str,
    eps: &[foundry_local_sdk::EpInfo],
) -> AppResult<Vec<String>> {
    let matching: Vec<String> = eps
        .iter()
        .filter(|ep| {
            let _ = model_alias;
            super::hardware::is_cuda_runtime_name(&ep.name)
        })
        .map(|ep| ep.name.clone())
        .collect();

    if matching.is_empty() {
        let available: Vec<&str> = eps.iter().map(|ep| ep.name.as_str()).collect();
        return Err(AppError::Message(format!(
            "Foundry did not report a CUDA execution provider for '{model_alias}'. Available EPs: {available:?}. CPU and WebGPU providers are not supported."
        )));
    }

    Ok(matching)
}

pub(crate) fn select_foundry_model_variant(
    model: &foundry_local_sdk::Model,
    model_alias: &str,
) -> AppResult<super::FoundryVariantChoice> {
    let choices: Vec<super::FoundryVariantChoice> = model
        .variants()
        .iter()
        .map(|variant| foundry_variant_choice_from_model(variant))
        .collect();
    let selected = select_foundry_vision_variant(model_alias, &choices)?;
    model.select_variant_by_id(&selected.id)?;
    Ok(selected)
}

pub(crate) fn foundry_variant_choice_from_model(
    model: &foundry_local_sdk::Model,
) -> super::FoundryVariantChoice {
    let info = model.info();
    super::FoundryVariantChoice {
        id: info.id.clone(),
        provider_type: info.provider_type.clone(),
        execution_provider: info
            .runtime
            .as_ref()
            .map(|runtime| runtime.execution_provider.clone()),
        device_type: info
            .runtime
            .as_ref()
            .map(|runtime| format!("{:?}", runtime.device_type)),
        input_modalities: info.input_modalities.clone().unwrap_or_default(),
        cached: info.cached,
    }
}

pub(crate) fn select_foundry_vision_variant(
    model_alias: &str,
    variants: &[super::FoundryVariantChoice],
) -> AppResult<super::FoundryVariantChoice> {
    let mut candidates: Vec<super::FoundryVariantChoice> = variants
        .iter()
        .filter(|variant| foundry_variant_accepts_images(variant))
        .filter(|variant| !foundry_variant_is_cpu(variant))
        .filter(|variant| !foundry_variant_is_webgpu(variant))
        .filter(|variant| {
            let _ = model_alias;
            foundry_variant_is_cuda(variant)
        })
        .cloned()
        .collect();

    candidates
        .sort_by_key(|variant| std::cmp::Reverse(foundry_variant_score(model_alias, variant)));
    if let Some(selected) = candidates.into_iter().next() {
        return Ok(selected);
    }

    let available = variants
        .iter()
        .map(describe_foundry_variant)
        .collect::<Vec<_>>()
        .join("; ");
    Err(AppError::Message(format!(
        "No supported GPU vision variant was found for '{model_alias}'. Expected CUDA; CPU and WebGPU variants are not supported. Available variants: {available}"
    )))
}

pub(crate) fn foundry_variant_score(
    model_alias: &str,
    variant: &super::FoundryVariantChoice,
) -> i32 {
    let mut score = 0;
    if variant.cached {
        score += 100;
    }
    if super::hardware::is_cuda_model_alias(model_alias) && foundry_variant_is_cuda(variant) {
        score += 50;
    }
    if contains_any(&variant.id, &["gpu", "npu"]) {
        score += 10;
    }
    score
}

pub(crate) fn foundry_variant_accepts_images(variant: &super::FoundryVariantChoice) -> bool {
    variant
        .input_modalities
        .split(',')
        .map(str::trim)
        .any(|modality| modality.eq_ignore_ascii_case("image"))
}

pub(crate) fn foundry_variant_is_cpu(variant: &super::FoundryVariantChoice) -> bool {
    contains_any(&variant.id, &["cpu"])
        || contains_any(&variant.provider_type, &["cpu"])
        || variant
            .device_type
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("CPU"))
            .unwrap_or(false)
        || variant
            .execution_provider
            .as_deref()
            .map(|value| contains_any(value, &["cpu", "mlas"]))
            .unwrap_or(false)
}

pub(crate) fn foundry_variant_is_webgpu(variant: &super::FoundryVariantChoice) -> bool {
    contains_any(&variant.id, &["webgpu", "web-gpu"])
        || contains_any(&variant.provider_type, &["webgpu", "web-gpu"])
        || variant
            .execution_provider
            .as_deref()
            .map(|value| contains_any(value, &["webgpu", "web-gpu"]))
            .unwrap_or(false)
}

pub(crate) fn foundry_variant_is_cuda(variant: &super::FoundryVariantChoice) -> bool {
    contains_any(&variant.id, &["cuda", "nvidia"])
        || contains_any(&variant.provider_type, &["cuda", "nvidia"])
        || variant
            .execution_provider
            .as_deref()
            .map(super::hardware::is_cuda_runtime_name)
            .unwrap_or(false)
}

pub(crate) fn contains_any(value: &str, needles: &[&str]) -> bool {
    let value = value.to_lowercase();
    needles.iter().any(|needle| value.contains(needle))
}

pub(crate) fn describe_foundry_variant(variant: &super::FoundryVariantChoice) -> String {
    format!(
        "{} provider={} runtime={}/{} modalities={} cached={}",
        variant.id,
        variant.provider_type,
        variant.device_type.as_deref().unwrap_or("unknown-device"),
        variant
            .execution_provider
            .as_deref()
            .unwrap_or("unknown-ep"),
        variant.input_modalities,
        variant.cached
    )
}

pub(crate) async fn prepare_foundry_execution_providers_for_live_run(
    manager: &FoundryLocalManager,
    model_alias: &str,
) -> AppResult<()> {
    let eps = manager.discover_eps()?;
    let target_names = select_required_execution_provider_names(model_alias, &eps)?;
    if target_names.iter().all(|name| {
        eps.iter()
            .find(|ep| ep.name == *name)
            .map(|ep| ep.is_registered)
            .unwrap_or(false)
    }) {
        log_event(
            "OCR",
            "Required Foundry execution providers are registered.",
        );
        return Ok(());
    }

    let refs: Vec<&str> = target_names.iter().map(String::as_str).collect();
    let register_start = Instant::now();
    let last_progress: std::sync::Arc<Mutex<HashMap<String, u8>>> =
        std::sync::Arc::new(Mutex::new(HashMap::new()));
    let progress_state = std::sync::Arc::clone(&last_progress);
    let result = manager
        .download_and_register_eps_with_progress(Some(&refs), move |ep_name, percent| {
            let rounded = percent.round().clamp(0.0, 100.0) as u8;
            let should_emit = progress_state
                .lock()
                .map(|mut state| {
                    let previous = state.insert(ep_name.to_string(), rounded);
                    previous != Some(rounded)
                })
                .unwrap_or(true);
            if should_emit {
                log_event(
                    "OCR",
                    format!("Registering Foundry EP {ep_name}: {rounded}%"),
                );
            }
        })
        .await?;

    if !result.success {
        let cuda_runtime = super::hardware::find_cuda_runtime_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not detected".to_string());
        return Err(AppError::Message(format!(
            "Foundry could not register required execution providers {:?}: {}. Failed: {:?}. CUDA runtime directory: {}",
            target_names, result.status, result.failed_eps, cuda_runtime
        )));
    }

    log_event(
        "OCR",
        format!(
            "Foundry execution providers registered in {:.2}s.",
            register_start.elapsed().as_secs_f64()
        ),
    );
    Ok(())
}
