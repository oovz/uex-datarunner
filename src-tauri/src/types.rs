use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::time::SystemTime;

pub(crate) const DEFAULT_AI_MODEL: &str = "qwen3-vl-4b-instruct";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppConfig {
    pub(crate) screenshot_dir: String,
    pub(crate) secret_key: String,
    pub(crate) delete_after_submit: bool,
    pub(crate) is_production: bool,
    #[serde(default = "default_data_type")]
    pub(crate) data_type: String,
    #[serde(default = "default_ai_model")]
    pub(crate) ai_model: String,
    #[serde(default)]
    pub(crate) keep_model_loaded: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            screenshot_dir: default_screenshot_dir(),
            secret_key: String::new(),
            delete_after_submit: false,
            is_production: true,
            data_type: default_data_type(),
            ai_model: default_ai_model(),
            keep_model_loaded: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreenshotFile {
    pub(crate) path: String,
    pub(crate) filename: String,
    pub(crate) modified_at_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessedScreenshot {
    pub(crate) file: ScreenshotFile,
    pub(crate) ocr_text: String,
    pub(crate) screenshot_base64: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessScreenshotsResult {
    pub(crate) screenshots: Vec<ProcessedScreenshot>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UexAccountCheck {
    pub(crate) can_submit: bool,
    pub(crate) label: Option<String>,
    pub(crate) reason: Option<String>,
    pub(crate) raw_status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrStatus {
    pub(crate) is_ready: bool,
    pub(crate) source: String,
    pub(crate) path: Option<String>,
    pub(crate) message: String,
    pub(crate) gpu_name: Option<String>,
    pub(crate) gpu_vendor: String,
    pub(crate) is_model_loaded: bool,
    pub(crate) loaded_model_id: Option<String>,
    pub(crate) selected_model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrMessageData {
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrBatchStartedData {
    pub(crate) total: usize,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrScreenshotStartedData {
    pub(crate) path: String,
    pub(crate) filename: String,
    pub(crate) index: usize,
    pub(crate) total: usize,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrScreenshotSucceededData {
    pub(crate) path: String,
    pub(crate) filename: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrScreenshotFailedData {
    pub(crate) path: String,
    pub(crate) filename: String,
    pub(crate) error: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrFinishedData {
    pub(crate) processed: usize,
    pub(crate) warnings: usize,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub(crate) enum OcrProgressEvent {
    Message(OcrMessageData),
    BatchStarted(OcrBatchStartedData),
    ScreenshotStarted(OcrScreenshotStartedData),
    ScreenshotSucceeded(OcrScreenshotSucceededData),
    ScreenshotFailed(OcrScreenshotFailedData),
    Finished(OcrFinishedData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TerminalCache {
    pub(crate) fetched_at_ms: u64,
    pub(crate) game_version: String,
    pub(crate) terminals: Vec<UexTerminal>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminalCachePayload {
    pub(crate) game_version: String,
    pub(crate) terminals: Vec<UexTerminal>,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CommodityCache {
    pub(crate) fetched_at_ms: u64,
    pub(crate) game_version: String,
    pub(crate) commodities: Vec<UexCommodity>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommodityCachePayload {
    pub(crate) game_version: String,
    pub(crate) commodities: Vec<UexCommodity>,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DataParametersCache {
    pub(crate) fetched_at_ms: u64,
    pub(crate) game_version: String,
    pub(crate) parameters: UexDataParameters,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataParametersCachePayload {
    pub(crate) game_version: String,
    pub(crate) parameters: UexDataParameters,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UexCommodity {
    pub(crate) id: i64,
    pub(crate) id_parent: Option<i64>,
    pub(crate) name: String,
    pub(crate) code: Option<String>,
    pub(crate) slug: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) weight_scu: Option<f64>,
    pub(crate) price_buy: Option<f64>,
    pub(crate) price_sell: Option<f64>,
    pub(crate) is_available: Option<i64>,
    pub(crate) is_available_live: Option<i64>,
    pub(crate) is_visible: Option<i64>,
    pub(crate) is_buyable: Option<i64>,
    pub(crate) is_sellable: Option<i64>,
    pub(crate) is_temporary: Option<i64>,
    pub(crate) is_illegal: Option<i64>,
    pub(crate) date_added: Option<i64>,
    pub(crate) date_modified: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UexTerminal {
    pub(crate) id: i64,
    pub(crate) id_star_system: Option<i64>,
    pub(crate) id_planet: Option<i64>,
    pub(crate) id_orbit: Option<i64>,
    pub(crate) id_moon: Option<i64>,
    pub(crate) id_space_station: Option<i64>,
    pub(crate) id_outpost: Option<i64>,
    pub(crate) id_poi: Option<i64>,
    pub(crate) id_city: Option<i64>,
    pub(crate) id_faction: Option<i64>,
    pub(crate) id_company: Option<i64>,
    pub(crate) name: String,
    pub(crate) fullname: Option<String>,
    pub(crate) nickname: Option<String>,
    pub(crate) displayname: Option<String>,
    pub(crate) code: Option<String>,
    #[serde(rename = "type")]
    pub(crate) terminal_type: Option<String>,
    pub(crate) contact_url: Option<String>,
    pub(crate) screenshot: Option<String>,
    pub(crate) screenshot_full: Option<String>,
    pub(crate) screenshot_author: Option<String>,
    pub(crate) is_available: Option<i64>,
    pub(crate) is_available_live: Option<i64>,
    pub(crate) is_visible: Option<i64>,
    pub(crate) is_default_system: Option<i64>,
    pub(crate) is_refinery: Option<i64>,
    pub(crate) is_cargo_center: Option<i64>,
    pub(crate) is_shop_fps: Option<i64>,
    pub(crate) is_shop_vehicle: Option<i64>,
    pub(crate) is_refuel: Option<i64>,
    pub(crate) is_repair: Option<i64>,
    pub(crate) is_nqa: Option<i64>,
    pub(crate) is_player_owned: Option<i64>,
    pub(crate) is_auto_load: Option<i64>,
    pub(crate) has_loading_dock: Option<i64>,
    pub(crate) has_docking_port: Option<i64>,
    pub(crate) has_freight_elevator: Option<i64>,
    pub(crate) game_version: Option<String>,
    pub(crate) date_added: Option<i64>,
    pub(crate) date_modified: Option<i64>,
    pub(crate) star_system_name: Option<String>,
    pub(crate) planet_name: Option<String>,
    pub(crate) orbit_name: Option<String>,
    pub(crate) moon_name: Option<String>,
    pub(crate) space_station_name: Option<String>,
    pub(crate) outpost_name: Option<String>,
    pub(crate) city_name: Option<String>,
    pub(crate) faction_name: Option<String>,
    pub(crate) company_name: Option<String>,
    pub(crate) max_container_size: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct UexDataParameters {
    pub(crate) is_accepting_reports: Option<i64>,
    pub(crate) is_accepting_ptu_reports: Option<i64>,
    pub(crate) is_datacenter_enabled: Option<i64>,
    pub(crate) game_version: Option<String>,
    pub(crate) game_version_ptu: Option<String>,
    pub(crate) is_accepted: Option<i64>,
    pub(crate) is_temporary_enabled: Option<i64>,
    pub(crate) price_variation: Option<i64>,
    pub(crate) scu_variation: Option<i64>,
    pub(crate) ttl: Option<i64>,
    pub(crate) notification: Option<String>,
}

pub(crate) fn default_screenshot_dir() -> String {
    std::env::var("USERPROFILE")
        .map(|profile| Path::new(&profile).join("Pictures").join("Star Citizen"))
        .ok()
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub(crate) fn default_data_type() -> String {
    "commodity".to_string()
}

pub(crate) fn default_ai_model() -> String {
    DEFAULT_AI_MODEL.to_string()
}

pub(crate) fn system_time_to_ms(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
