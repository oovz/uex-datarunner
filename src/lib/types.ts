export type UexDatarunnerDataType = "commodity";

export type CommodityRow = {
  id: string;
  screenshotPath: string | null;
  commodityName: string;
  idCommodity: number | null;
  priceBuy: number | null;
  scuBuy: number | null;
  statusBuy: number | null;
  priceSell: number | null;
  scuSell: number | null;
  statusSell: number | null;
  cargoSizes: number[];
  sourceLines: string[];
  confidence: number;
  issues: string[];
};

export type ScreenshotFile = {
  path: string;
  filename: string;
  modifiedAtMs: number;
};

export type AppConfig = {
  screenshotDir: string;
  secretKey: string;
  deleteAfterSubmit: boolean;
  isProduction: boolean;
  dataType: UexDatarunnerDataType;
  aiModel: string;
  keepModelLoaded: boolean;
};

export type ProcessedScreenshot = {
  file: ScreenshotFile;
  ocrText: string;
  screenshotBase64: string;
};

export type OcrScreenshotStatus = "queued" | "processing" | "done" | "failed";

export type OcrScreenshotState = {
  status: OcrScreenshotStatus;
  message: string;
};

export type OcrProgressEvent =
  | { event: "message"; data: { message: string } }
  | { event: "batchStarted"; data: { total: number; message: string } }
  | {
      event: "screenshotStarted";
      data: { path: string; filename: string; index: number; total: number; message: string };
    }
  | { event: "screenshotSucceeded"; data: { path: string; filename: string; message: string } }
  | {
      event: "screenshotFailed";
      data: { path: string; filename: string; error: string; message: string };
    }
  | { event: "finished"; data: { processed: number; warnings: number; message: string } };

/**
 * The in-progress review session persisted by the backend so the user's
 * extracted commodities and processed screenshots survive minimize, tray-hide,
 * and full restarts until they submit or clear.
 */
export type WorkingSet = {
  version: 1;
  rows: CommodityRow[];
  screenshots: ProcessedScreenshot[];
  terminalId: number | null;
  gameVersion: string;
};

export type UexTerminal = {
  id: number;
  id_star_system?: number | null;
  id_planet?: number | null;
  id_orbit?: number | null;
  id_moon?: number | null;
  id_space_station?: number | null;
  id_outpost?: number | null;
  id_poi?: number | null;
  id_city?: number | null;
  id_faction?: number | null;
  id_company?: number | null;
  name: string;
  fullname?: string | null;
  nickname?: string | null;
  displayname: string | null;
  code: string | null;
  type?: string | null;
  contact_url?: string | null;
  screenshot?: string | null;
  screenshot_full?: string | null;
  screenshot_author?: string | null;
  is_available?: number | null;
  is_available_live?: number | null;
  is_visible?: number | null;
  is_default_system?: number | null;
  is_refinery?: number | null;
  is_cargo_center?: number | null;
  is_shop_fps?: number | null;
  is_shop_vehicle?: number | null;
  is_refuel?: number | null;
  is_repair?: number | null;
  is_nqa?: number | null;
  is_player_owned?: number | null;
  is_auto_load?: number | null;
  has_loading_dock?: number | null;
  has_docking_port?: number | null;
  has_freight_elevator?: number | null;
  game_version?: string | null;
  date_added?: number | null;
  date_modified?: number | null;
  star_system_name?: string | null;
  city_name: string | null;
  planet_name: string | null;
  orbit_name?: string | null;
  moon_name: string | null;
  space_station_name: string | null;
  outpost_name: string | null;
  faction_name?: string | null;
  company_name?: string | null;
  max_container_size?: unknown;
};

export type UexCommodity = {
  id: number;
  id_parent?: number | null;
  name: string;
  code: string | null;
  slug?: string | null;
  kind?: string | null;
  weight_scu?: number | null;
  price_buy?: number | null;
  price_sell?: number | null;
  is_available?: number | null;
  is_available_live?: number | null;
  is_visible?: number | null;
  is_buyable?: number | null;
  is_sellable?: number | null;
  is_temporary?: number | null;
  is_illegal?: number | null;
  date_added?: number | null;
  date_modified?: number | null;
};

export type UexDataParameters = {
  is_accepting_reports?: number | null;
  is_accepting_ptu_reports?: number | null;
  is_datacenter_enabled?: number | null;
  game_version?: string | null;
  game_version_ptu?: string | null;
  is_accepted?: number | null;
  is_temporary_enabled?: number | null;
  price_variation?: number | null;
  scu_variation?: number | null;
  ttl?: number | null;
  notification?: string | null;
};

export type UexAccountCheck = {
  canSubmit: boolean;
  label: string | null;
  reason: string | null;
  rawStatus?: string;
};
