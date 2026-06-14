use std::{
    fs,
    path::PathBuf,
    sync::{atomic::AtomicBool, Mutex},
};

use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};
use crate::logging::log_event;
use crate::types::AppConfig;

pub(crate) struct AppState {
    pub(crate) config_cache: Mutex<Option<AppConfig>>,
    pub(crate) registered_execution_providers: Mutex<std::collections::HashSet<String>>,
    pub(crate) is_exiting: AtomicBool,
    pub(crate) is_ocr_cancelled: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config_cache: Mutex::new(None),
            registered_execution_providers: Mutex::new(std::collections::HashSet::new()),
            is_exiting: AtomicBool::new(false),
            is_ocr_cancelled: AtomicBool::new(false),
        }
    }
}

pub(crate) fn read_config(app: &AppHandle) -> AppResult<AppConfig> {
    let state = app.state::<AppState>();
    if let Ok(cache) = state.config_cache.lock() {
        if let Some(ref config) = *cache {
            return Ok(config.clone());
        }
    }

    let path = config_path(app)?;
    let config = if !path.exists() {
        let config = AppConfig::default();
        write_config(app, &config)?;
        config
    } else {
        let contents = fs::read_to_string(&path)?;
        serde_json::from_str(&contents)?
    };

    if let Ok(mut cache) = state.config_cache.lock() {
        *cache = Some(config.clone());
    }
    Ok(config)
}

pub(crate) fn write_config(app: &AppHandle, config: &AppConfig) -> AppResult<()> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(config)?)?;

    let state = app.state::<AppState>();
    if let Ok(mut cache) = state.config_cache.lock() {
        *cache = Some(config.clone());
    }
    Ok(())
}

pub(crate) fn config_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| AppError::Message(error.to_string()))?
        .join("config.json"))
}

pub(crate) fn working_set_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Message(error.to_string()))?
        .join("working-set.json"))
}

pub(crate) fn terminal_cache_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|error| AppError::Message(error.to_string()))?
        .join("terminal-cache.json"))
}

pub(crate) fn commodity_cache_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|error| AppError::Message(error.to_string()))?
        .join("commodity-cache.json"))
}

pub(crate) fn data_parameters_cache_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|error| AppError::Message(error.to_string()))?
        .join("data-parameters-cache.json"))
}

pub(crate) fn migrate_legacy_caches(app: &AppHandle) -> AppResult<()> {
    const OLD_IDENTIFIER: &str = "space.uexcorp.datarunner";
    let current_cache = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Message(e.to_string()))?;
    let Some(cache_parent) = current_cache.parent() else {
        return Ok(());
    };
    let old_cache = cache_parent.join(OLD_IDENTIFIER).join("cache");
    if !old_cache.exists() {
        return Ok(());
    }

    let files = [
        ("commodity-cache.json", commodity_cache_path(app)?),
        ("terminal-cache.json", terminal_cache_path(app)?),
        (
            "data-parameters-cache.json",
            data_parameters_cache_path(app)?,
        ),
    ];

    for (filename, new_path) in &files {
        let old_path = old_cache.join(filename);
        if old_path.exists() && !new_path.exists() {
            if let Some(parent) = new_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(e) = fs::copy(&old_path, new_path) {
                log_event("migrate", format!("Failed to copy {}: {}", filename, e));
            } else {
                log_event("migrate", format!("Copied {} from legacy cache", filename));
            }
        }
    }
    Ok(())
}
