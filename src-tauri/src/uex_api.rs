use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::time::SystemTime;
use tauri::AppHandle;

use crate::error::{AppError, AppResult};
use crate::logging::log_event;
use crate::state::{commodity_cache_path, data_parameters_cache_path, terminal_cache_path};
use crate::types::*;

pub(crate) const UEX_API_BASE: &str = "https://api.uexcorp.uk/2.0";

pub(crate) fn classify_uex_account(response: Value) -> UexAccountCheck {
    let raw_status = response
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let data = response.get("data");
    let label = data
        .and_then(|value| value.get("username"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    if raw_status == "invalid_secret_key" {
        return UexAccountCheck {
            can_submit: false,
            label,
            reason: Some("UEX rejected this secret key.".to_string()),
            raw_status,
        };
    }

    if raw_status == "user_not_allowed" {
        return UexAccountCheck {
            can_submit: false,
            label,
            reason: Some("This UEX account is disabled or not allowed.".to_string()),
            raw_status,
        };
    }

    if raw_status != "ok" {
        return UexAccountCheck {
            can_submit: false,
            label,
            reason: Some("UEX account check did not return an eligible user.".to_string()),
            raw_status,
        };
    }

    let is_datarunner = data
        .and_then(|value| value.get("is_datarunner"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let is_banned = data
        .and_then(|value| value.get("is_datarunner_banned"))
        .and_then(Value::as_i64)
        .unwrap_or(0);

    if is_banned == 1 {
        return UexAccountCheck {
            can_submit: false,
            label,
            reason: Some("This UEX datarunner account is banned.".to_string()),
            raw_status,
        };
    }

    if is_datarunner != 1 {
        return UexAccountCheck {
            can_submit: false,
            label,
            reason: Some("This UEX account is not enabled as a datarunner.".to_string()),
            raw_status,
        };
    }

    UexAccountCheck {
        can_submit: true,
        label,
        reason: None,
        raw_status,
    }
}

pub(crate) async fn get_cached_terminals(
    app: &AppHandle,
    force: bool,
) -> AppResult<TerminalCachePayload> {
    let cached = read_terminal_cache(app)?;

    if !force {
        if let Some(ref cache) = cached {
            return Ok(TerminalCachePayload {
                game_version: cache.game_version.clone(),
                terminals: cache.terminals.clone(),
                source: "cache".to_string(),
            });
        }
    }

    match fetch_terminals_from_api("commodity").await {
        Ok(terminals) => {
            let game_version = fetch_current_game_version()
                .await
                .unwrap_or_else(|_| "unknown".to_string());
            if !terminals.is_empty() {
                write_terminal_cache(app, &game_version, &terminals)?;
            }
            Ok(TerminalCachePayload {
                game_version,
                terminals,
                source: "api".to_string(),
            })
        }
        Err(error) => {
            log_event("cache", format!("Terminal fetch failed: {}", error));
            if let Some(cache) = cached {
                Ok(TerminalCachePayload {
                    game_version: cache.game_version,
                    terminals: cache.terminals,
                    source: "stale-cache".to_string(),
                })
            } else {
                Ok(TerminalCachePayload {
                    game_version: "unknown".to_string(),
                    terminals: vec![],
                    source: "unavailable".to_string(),
                })
            }
        }
    }
}

pub(crate) async fn get_cached_commodities(
    app: &AppHandle,
    force: bool,
) -> AppResult<CommodityCachePayload> {
    let cached = read_commodity_cache(app)?;

    if !force {
        if let Some(ref cache) = cached {
            return Ok(CommodityCachePayload {
                game_version: cache.game_version.clone(),
                commodities: cache.commodities.clone(),
                source: "cache".to_string(),
            });
        }
    }

    match fetch_commodities_from_api().await {
        Ok(commodities) => {
            let game_version = fetch_current_game_version()
                .await
                .unwrap_or_else(|_| "unknown".to_string());
            if !commodities.is_empty() {
                write_commodity_cache(app, &game_version, &commodities)?;
            } else {
                log_event(
                    "cache",
                    "Commodity API returned empty list; not writing cache",
                );
            }
            Ok(CommodityCachePayload {
                game_version,
                commodities,
                source: "api".to_string(),
            })
        }
        Err(error) => {
            log_event("cache", format!("Commodity fetch failed: {}", error));
            if let Some(cache) = cached {
                Ok(CommodityCachePayload {
                    game_version: cache.game_version,
                    commodities: cache.commodities,
                    source: "stale-cache".to_string(),
                })
            } else {
                Ok(CommodityCachePayload {
                    game_version: "unknown".to_string(),
                    commodities: vec![],
                    source: "unavailable".to_string(),
                })
            }
        }
    }
}

pub(crate) async fn get_cached_data_parameters(
    app: &AppHandle,
    force: bool,
) -> AppResult<DataParametersCachePayload> {
    let cached = read_data_parameters_cache(app)?;

    if !force {
        if let Some(ref cache) = cached {
            return Ok(DataParametersCachePayload {
                game_version: cache.game_version.clone(),
                parameters: cache.parameters.clone(),
                source: "cache".to_string(),
            });
        }
    }

    match fetch_data_parameters_from_api().await {
        Ok(parameters) => {
            let game_version = parameters
                .game_version
                .as_deref()
                .filter(|version| !version.trim().is_empty())
                .unwrap_or("unknown");
            write_data_parameters_cache(app, game_version, &parameters)?;
            Ok(DataParametersCachePayload {
                game_version: game_version.to_string(),
                parameters,
                source: "api".to_string(),
            })
        }
        Err(error) => {
            log_event("cache", format!("Data parameters fetch failed: {}", error));
            if let Some(cache) = cached {
                Ok(DataParametersCachePayload {
                    game_version: cache.game_version,
                    parameters: cache.parameters,
                    source: "stale-cache".to_string(),
                })
            } else {
                Ok(DataParametersCachePayload {
                    game_version: "unknown".to_string(),
                    parameters: UexDataParameters::default(),
                    source: "unavailable".to_string(),
                })
            }
        }
    }
}

async fn fetch_current_game_version() -> AppResult<String> {
    log_event("api", "Fetching current game version from UEX");
    let response = reqwest::Client::new()
        .get(format!("{UEX_API_BASE}/game_versions"))
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    extract_live_game_version(&response)
}

pub(crate) fn extract_live_game_version(response: &Value) -> AppResult<String> {
    let data = response.get("data").unwrap_or(response);
    let live = data
        .get("live")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    if live.is_empty() {
        return Err(AppError::Message(
            "UEX game_versions did not return a live game version.".to_string(),
        ));
    }

    Ok(live.to_string())
}

async fn fetch_terminals_from_api(data_type: &str) -> AppResult<Vec<UexTerminal>> {
    log_event("api", format!("Fetching terminals (type={})", data_type));
    let response = reqwest::Client::new()
        .get(format!("{UEX_API_BASE}/terminals"))
        .query(&[("type", data_type)])
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    parse_uex_data_array(response, "terminals")
}

async fn fetch_commodities_from_api() -> AppResult<Vec<UexCommodity>> {
    log_event("api", "Fetching commodities");
    let response = reqwest::Client::new()
        .get(format!("{UEX_API_BASE}/commodities"))
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    parse_uex_data_array(response, "commodities")
}

async fn fetch_data_parameters_from_api() -> AppResult<UexDataParameters> {
    log_event("api", "Fetching data parameters");
    let response = reqwest::Client::new()
        .get(format!("{UEX_API_BASE}/data_parameters"))
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    parse_uex_data_object(response, "data_parameters")
}

pub(crate) fn parse_uex_data_array<T>(response: Value, endpoint: &str) -> AppResult<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let data = response.get("data").ok_or_else(|| {
        AppError::Message(format!(
            "UEX {endpoint} response did not include a data array."
        ))
    })?;
    let values = data.as_array().ok_or_else(|| {
        AppError::Message(format!("UEX {endpoint} response data was not an array."))
    })?;

    values
        .iter()
        .cloned()
        .map(serde_json::from_value)
        .collect::<Result<Vec<T>, _>>()
        .map_err(AppError::from)
}

pub(crate) fn parse_uex_data_object<T>(response: Value, endpoint: &str) -> AppResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    let data = response.get("data").ok_or_else(|| {
        AppError::Message(format!(
            "UEX {endpoint} response did not include a data object."
        ))
    })?;
    if !data.is_object() {
        return Err(AppError::Message(format!(
            "UEX {endpoint} response data was not an object."
        )));
    }

    Ok(serde_json::from_value(data.clone())?)
}

fn read_terminal_cache(app: &AppHandle) -> AppResult<Option<TerminalCache>> {
    let path = terminal_cache_path(app)?;
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&contents)?))
}

fn write_terminal_cache(
    app: &AppHandle,
    game_version: &str,
    terminals: &[UexTerminal],
) -> AppResult<()> {
    let path = terminal_cache_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let cache = TerminalCache {
        fetched_at_ms: system_time_to_ms(SystemTime::now()),
        game_version: game_version.to_string(),
        terminals: terminals.to_vec(),
    };
    fs::write(path, serde_json::to_string_pretty(&cache)?)?;
    Ok(())
}

fn read_commodity_cache(app: &AppHandle) -> AppResult<Option<CommodityCache>> {
    let path = commodity_cache_path(app)?;
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&contents)?))
}

fn write_commodity_cache(
    app: &AppHandle,
    game_version: &str,
    commodities: &[UexCommodity],
) -> AppResult<()> {
    let path = commodity_cache_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let cache = CommodityCache {
        fetched_at_ms: system_time_to_ms(SystemTime::now()),
        game_version: game_version.to_string(),
        commodities: commodities.to_vec(),
    };
    fs::write(path, serde_json::to_string_pretty(&cache)?)?;
    Ok(())
}

fn read_data_parameters_cache(app: &AppHandle) -> AppResult<Option<DataParametersCache>> {
    let path = data_parameters_cache_path(app)?;
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&contents)?))
}

fn write_data_parameters_cache(
    app: &AppHandle,
    game_version: &str,
    parameters: &UexDataParameters,
) -> AppResult<()> {
    let path = data_parameters_cache_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let cache = DataParametersCache {
        fetched_at_ms: system_time_to_ms(SystemTime::now()),
        game_version: game_version.to_string(),
        parameters: parameters.clone(),
    };
    fs::write(path, serde_json::to_string_pretty(&cache)?)?;
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn filter_terminals(
    terminals: Vec<UexTerminal>,
    query: &str,
    limit: usize,
) -> Vec<UexTerminal> {
    let query = query.trim().to_lowercase();
    terminals
        .into_iter()
        .filter(|terminal| {
            if query.is_empty() {
                return true;
            }
            terminal_matches_query(terminal, &query)
        })
        .take(limit)
        .collect()
}

pub(crate) fn filter_commodities(
    commodities: Vec<UexCommodity>,
    query: &str,
    limit: usize,
) -> Vec<UexCommodity> {
    let query = query.trim().to_lowercase();
    commodities
        .into_iter()
        .filter(|commodity| {
            query.is_empty()
                || commodity.name.to_lowercase().contains(&query)
                || commodity
                    .code
                    .as_deref()
                    .map(|code| code.to_lowercase().contains(&query))
                    .unwrap_or(false)
                || commodity
                    .slug
                    .as_deref()
                    .map(|slug| slug.to_lowercase().contains(&query))
                    .unwrap_or(false)
        })
        .take(limit)
        .collect()
}

#[allow(dead_code)]
pub(crate) fn terminal_matches_query(terminal: &UexTerminal, query: &str) -> bool {
    [
        Some(terminal.name.as_str()),
        terminal.fullname.as_deref(),
        terminal.displayname.as_deref(),
        terminal.code.as_deref(),
        terminal.star_system_name.as_deref(),
        terminal.planet_name.as_deref(),
        terminal.orbit_name.as_deref(),
        terminal.moon_name.as_deref(),
        terminal.space_station_name.as_deref(),
        terminal.outpost_name.as_deref(),
        terminal.city_name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| value.to_lowercase().contains(query))
}
