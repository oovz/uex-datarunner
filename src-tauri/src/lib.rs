use chrono::Local;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};
use tauri_plugin_log::{Target, TargetKind};

mod error;
mod foundry_ocr;
mod logging;
mod ocr_events;
mod screenshots;
mod state;
mod types;
mod uex_api;

pub(crate) use error::*;
pub(crate) use logging::log_event;
pub(crate) use ocr_events::*;
pub(crate) use screenshots::*;
pub(crate) use state::*;
pub(crate) use types::*;
pub(crate) use uex_api::*;
#[cfg(test)]
pub(crate) use uex_api::{parse_uex_data_array, parse_uex_data_object};

#[cfg(test)]
pub(crate) use foundry_ocr::{
    commodity_extraction_schema, default_ai_model, ensure_foundry_model_accepts_images,
    find_cuda_runtime_dir_in_roots, foundry_ocr_messages, is_foundry_transport_failure,
    merge_foundry_ocr_results, normalize_foundry_ocr_json,
    prepare_foundry_execution_providers_for_live_run, prepare_right_panel_ocr_images_in,
    run_foundry_ocr_image, select_foundry_model_variant, select_foundry_vision_variant,
    strip_model_reasoning, validate_foundry_ocr_json, verify_foundry_service_health,
    FoundryVariantChoice, CUDA_VISION_MODEL_ALIAS, FOUNDRY_OCR_HTTP_TIMEOUT_SECS,
    FOUNDRY_OCR_MAX_TOKENS, QWEN3_VL_4B_INSTRUCT_CUDA_ALIAS,
};

use foundry_ocr::{
    check_foundry_ocr_model, configured_ai_model, foundry_loaded_model_status, get_system_gpu_info,
    is_cuda_runtime_available, process_files_with_foundry_ocr, validate_ocr_hardware_policy,
};

#[tauri::command]
fn load_config(app: AppHandle) -> AppResult<AppConfig> {
    read_config(&app)
}

#[tauri::command]
fn save_config(app: AppHandle, config: AppConfig) -> AppResult<AppConfig> {
    write_config(&app, &config)?;
    Ok(config)
}

#[tauri::command]
fn list_screenshots(app: AppHandle) -> AppResult<Vec<ScreenshotFile>> {
    let config = read_config(&app)?;
    let files = list_screenshot_files(Path::new(&config.screenshot_dir))?;
    log_event(
        "screenshots",
        format!("Listed {} screenshot(s)", files.len()),
    );
    Ok(files)
}

#[tauri::command]
fn open_screenshot(path: String) -> AppResult<()> {
    let path = PathBuf::from(path);
    if !path.is_file() || !is_supported_image(&path) {
        return Err(AppError::Message(
            "Select an existing screenshot image to open.".to_string(),
        ));
    }

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("explorer.exe");
        command.arg(&path);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(&path);
        command
    };

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(&path);
        command
    };

    command.spawn()?;
    Ok(())
}

#[tauri::command]
async fn process_screenshots(app: AppHandle) -> AppResult<ProcessScreenshotsResult> {
    app.state::<AppState>()
        .is_ocr_cancelled
        .store(false, Ordering::Relaxed);

    let config = read_config(&app)?;
    if config.screenshot_dir.trim().is_empty() {
        return Err(AppError::Message(
            "Choose a screenshot directory first".to_string(),
        ));
    }

    let files = list_screenshot_files(Path::new(&config.screenshot_dir))?;

    log_event(
        "process_screenshots",
        format!("Found {} screenshot(s)", files.len()),
    );
    emit_ocr_progress(&app, &format!("Found {} screenshot(s)", files.len()));

    let model_alias = configured_ai_model(&config);
    validate_ocr_hardware_policy(&model_alias)?;
    process_files_with_foundry_ocr(&app, &files, &model_alias).await
}

#[tauri::command]
async fn process_selected_screenshots(
    app: AppHandle,
    paths: Vec<String>,
) -> AppResult<ProcessScreenshotsResult> {
    app.state::<AppState>()
        .is_ocr_cancelled
        .store(false, Ordering::Relaxed);

    let config = read_config(&app)?;
    if config.screenshot_dir.trim().is_empty() {
        return Err(AppError::Message(
            "Choose a screenshot directory first".to_string(),
        ));
    }

    let all_files = list_screenshot_files(Path::new(&config.screenshot_dir))?;
    let path_set: HashSet<String> = paths.into_iter().collect();
    let selected: Vec<ScreenshotFile> = all_files
        .into_iter()
        .filter(|file| path_set.contains(&file.path))
        .collect();

    log_event(
        "process_selected_screenshots",
        format!("Selected {} screenshot(s)", selected.len()),
    );
    emit_ocr_progress(
        &app,
        &format!("Selected {} screenshot(s) for OCR", selected.len()),
    );

    let model_alias = configured_ai_model(&config);
    validate_ocr_hardware_policy(&model_alias)?;
    process_files_with_foundry_ocr(&app, &selected, &model_alias).await
}

#[tauri::command]
async fn check_uex_account(app: AppHandle) -> AppResult<UexAccountCheck> {
    let config = read_config(&app)?;
    if config.secret_key.trim().is_empty() {
        return Ok(UexAccountCheck {
            can_submit: false,
            label: None,
            reason: Some("Enter your UEX secret key first.".to_string()),
            raw_status: "missing_secret_key".to_string(),
        });
    }

    let response = reqwest::Client::new()
        .get(format!("{UEX_API_BASE}/user"))
        .header("secret-key", config.secret_key.trim())
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    let result = classify_uex_account(response);
    log_event(
        "uex",
        format!(
            "Account check: can_submit={} label={:?}",
            result.can_submit, result.label
        ),
    );
    Ok(result)
}

#[tauri::command]
async fn get_ocr_status(app: AppHandle) -> AppResult<OcrStatus> {
    let config = read_config(&app)?;
    let model_alias = configured_ai_model(&config);
    let (gpu_name, gpu_vendor) = get_system_gpu_info();

    if gpu_name.is_none() {
        return Ok(OcrStatus {
            is_ready: false,
            source: "foundryLocalCuda".to_string(),
            path: None,
            message: "No NVIDIA GPU detected. CPU-only and WebGPU execution are not supported."
                .to_string(),
            gpu_name,
            gpu_vendor: "CPU-Only (Unsupported)".to_string(),
            is_model_loaded: false,
            loaded_model_id: None,
            selected_model_id: None,
        });
    }

    let has_nvidia = gpu_vendor.to_uppercase().contains("NVIDIA");
    let has_cuda = is_cuda_runtime_available();
    if !has_nvidia || !has_cuda {
        return Ok(OcrStatus {
            is_ready: false,
            source: "foundryLocalCuda".to_string(),
            path: None,
            message: format!(
                "Foundry OCR model '{model_alias}' requires an NVIDIA GPU and an active CUDA runtime."
            ),
            gpu_name,
            gpu_vendor: if has_nvidia {
                "NVIDIA (No CUDA Runtime)".to_string()
            } else {
                gpu_vendor
            },
            is_model_loaded: false,
            loaded_model_id: None,
            selected_model_id: None,
        });
    }

    let display_vendor = if has_nvidia {
        if has_cuda {
            "NVIDIA CUDA (Active)".to_string()
        } else {
            "NVIDIA CUDA (Missing Runtime)".to_string()
        }
    } else {
        gpu_vendor
    };

    let (is_model_loaded, loaded_model_id) = foundry_loaded_model_status(&model_alias)
        .await
        .unwrap_or((false, None));
    match check_foundry_ocr_model(&model_alias).await {
        Ok(selected_model_id) => Ok(OcrStatus {
            is_ready: true,
            source: "foundryLocalCuda".to_string(),
            path: None,
            message: format!("{model_alias} is available through Foundry Local."),
            gpu_name,
            gpu_vendor: display_vendor,
            is_model_loaded,
            loaded_model_id,
            selected_model_id: Some(selected_model_id),
        }),
        Err(error) => Ok(OcrStatus {
            is_ready: false,
            source: "foundryLocalCuda".to_string(),
            path: None,
            message: error.to_string(),
            gpu_name,
            gpu_vendor: display_vendor,
            is_model_loaded,
            loaded_model_id,
            selected_model_id: None,
        }),
    }
}

#[tauri::command]
async fn prefetch_terminals(app: AppHandle, force: bool) -> AppResult<TerminalCachePayload> {
    get_cached_terminals(&app, force).await
}

#[tauri::command]
async fn prefetch_commodities(app: AppHandle, force: bool) -> AppResult<CommodityCachePayload> {
    get_cached_commodities(&app, force).await
}

#[tauri::command]
async fn prefetch_data_parameters(
    app: AppHandle,
    force: bool,
) -> AppResult<DataParametersCachePayload> {
    get_cached_data_parameters(&app, force).await
}

#[tauri::command]
fn delete_submitted_screenshots(app: AppHandle, paths: Vec<String>) -> AppResult<AppConfig> {
    let config = read_config(&app)?;
    if config.delete_after_submit {
        for path in &paths {
            if let Err(error) = fs::remove_file(path) {
                if error.kind() != io::ErrorKind::NotFound {
                    return Err(AppError::Io(error));
                }
            }
        }
    }
    Ok(config)
}

#[tauri::command]
async fn search_commodities(app: AppHandle, query: String) -> AppResult<Vec<UexCommodity>> {
    let payload = get_cached_commodities(&app, false).await?;
    Ok(filter_commodities(payload.commodities, &query, 80))
}

#[tauri::command]
async fn submit_to_uex(app: AppHandle, payload: Value) -> AppResult<Value> {
    let config = read_config(&app)?;
    if config.secret_key.trim().is_empty() {
        return Err(AppError::Message(
            "Save your UEX secret key before submitting".to_string(),
        ));
    }

    let row_count = payload
        .get("prices")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    log_event(
        "uex",
        format!(
            "POST /data_submit terminal={} type={} is_production={} rows={}",
            payload
                .get("id_terminal")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            payload.get("type").and_then(Value::as_str).unwrap_or("?"),
            payload
                .get("is_production")
                .and_then(Value::as_i64)
                .unwrap_or(-1),
            row_count,
        ),
    );

    let http_response = reqwest::Client::new()
        .post(format!("{UEX_API_BASE}/data_submit"))
        .header("secret-key", config.secret_key.trim())
        .json(&payload)
        .send()
        .await?;
    let http_status = http_response.status();
    let body = http_response.text().await?;
    let response: Value =
        serde_json::from_str(&body).unwrap_or_else(|_| json!({ "status": body.clone() }));
    let status = response.get("status").and_then(Value::as_str).unwrap_or("");

    let body_excerpt = body.chars().take(500).collect::<String>();
    if http_status.is_success() && status == "ok" {
        log_event("uex", format!("data_submit accepted: {body_excerpt}"));
    } else {
        log_event(
            "uex",
            format!("data_submit rejected (http={http_status}): {body_excerpt}"),
        );
    }
    Ok(response)
}

#[tauri::command]
async fn cancel_ocr(app: AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    state.is_ocr_cancelled.store(true, Ordering::Relaxed);
    emit_ocr_progress(&app, "OCR cancellation requested...");
    Ok(())
}

#[tauri::command]
fn hide_to_tray(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .hide()
            .map_err(|error| AppError::Message(error.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
fn load_working_set(app: AppHandle) -> AppResult<Value> {
    let path = working_set_path(&app)?;
    if !path.exists() {
        return Ok(Value::Null);
    }
    let contents = fs::read_to_string(&path)?;
    match serde_json::from_str::<Value>(&contents) {
        Ok(value) => {
            log_event("session", "Loaded persisted working set");
            Ok(value)
        }
        Err(error) => {
            log_event(
                "session",
                format!("Discarding unreadable working set: {error}"),
            );
            Ok(Value::Null)
        }
    }
}

#[tauri::command]
fn save_working_set(app: AppHandle, snapshot: Value) -> AppResult<()> {
    let path = working_set_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string(&snapshot)?)?;
    Ok(())
}

#[tauri::command]
fn clear_working_set(app: AppHandle) -> AppResult<()> {
    let path = working_set_path(&app)?;
    if path.exists() {
        fs::remove_file(&path)?;
        log_event("session", "Cleared working set");
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir {
                        file_name: Some("uex-datarunner".to_string()),
                    }),
                    Target::new(TargetKind::Webview),
                ])
                .level(log::LevelFilter::Info)
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{}][{}] {}",
                        Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                        record.target(),
                        message
                    ))
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            migrate_legacy_caches(app.app_handle())?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show UEX Datarunner", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        let state = app.state::<AppState>();
                        state.is_exiting.store(true, Ordering::Relaxed);
                        for window in app.webview_windows().values() {
                            let _ = window.close();
                        }
                    }
                    "show" => show_main_window(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            tray_builder.build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.is_exiting.load(Ordering::Relaxed) {
                    // Allow window to close during shutdown
                } else {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            list_screenshots,
            open_screenshot,
            process_screenshots,
            process_selected_screenshots,
            check_uex_account,
            get_ocr_status,
            prefetch_terminals,
            prefetch_commodities,
            prefetch_data_parameters,
            delete_submitted_screenshots,
            search_commodities,
            submit_to_uex,
            cancel_ocr,
            hide_to_tray,
            load_working_set,
            save_working_set,
            clear_working_set
        ])
        .run(tauri::generate_context!())
        .expect("error while running UEX Datarunner");
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests;
