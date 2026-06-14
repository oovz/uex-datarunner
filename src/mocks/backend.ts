import type {
  AppConfig,
  ProcessedScreenshot,
  ScreenshotFile,
  UexAccountCheck,
  UexCommodity,
  UexDataParameters,
  UexTerminal,
} from "../../src/lib/types";
import type { OcrStatus } from "../../src/lib/setup";
import { CUDA_VISION_MODEL_ALIAS } from "../../src/lib/ai-models";

export const mockAppConfig: AppConfig = {
  screenshotDir: "C:\\Mock\\StarCitizen\\screenshots",
  secretKey: "mock-secret-key",
  deleteAfterSubmit: false,
  isProduction: true,
  dataType: "commodity",
  aiModel: CUDA_VISION_MODEL_ALIAS,
  keepModelLoaded: false,
};

export const mockAccount: UexAccountCheck = {
  canSubmit: true,
  label: "MockDatarunner",
  reason: null,
  rawStatus: "ok",
};

export const mockOcrStatus: OcrStatus = {
  isReady: true,
  source: "foundryLocalCuda",
  path: null,
  message: "Foundry Local OCR is ready.",
  gpuName: "NVIDIA GeForce RTX 4090",
  gpuVendor: "NVIDIA CUDA (Active)",
  isModelLoaded: false,
  loadedModelId: null,
  selectedModelId: "qwen3.5-4b-cuda-gpu:2",
};

export const mockTerminals: UexTerminal[] = [
  {
    id: 89,
    name: "Area18 TDD",
    fullname: "Area18 Trade and Development Division",
    displayname: "Area18 TDD",
    code: "AREA18-TDD",
    type: "commodity",
    star_system_name: "Stanton",
    city_name: "Area18",
    planet_name: "ArcCorp",
    moon_name: null,
    space_station_name: null,
    outpost_name: null,
  },
];

export const mockCommodities: UexCommodity[] = [
  {
    id: 1,
    id_parent: null,
    name: "Agricium",
    code: "AGRI",
    slug: "agricium",
  },
  {
    id: 7,
    id_parent: null,
    name: "Processed Food",
    code: "FOOD",
    slug: "processed-food",
  },
];

export const mockDataParameters: UexDataParameters = {
  is_accepting_reports: 1,
  is_accepting_ptu_reports: 1,
  is_datacenter_enabled: 1,
  game_version: "4.2.1",
  game_version_ptu: "4.2.1-PTU",
};

export const mockPendingScreenshots: ScreenshotFile[] = [
  {
    path: "C:\\Mock\\StarCitizen\\screenshots\\ScreenShot-0001.jpg",
    filename: "ScreenShot-0001.jpg",
    modifiedAtMs: 1_700_000_000_000,
  },
];

export const mockProcessedScreenshots: ProcessedScreenshot[] = [
  {
    file: mockPendingScreenshots[0],
    screenshotBase64: "mock-screenshot-base64",
    // Structured JSON is what the OCR backend now returns (the model is asked
    // for structured output and any <think> block is stripped before parsing).
    ocrText: JSON.stringify({
      marketSide: "sell",
      commodities: [
        { name: "Agricium", status: "high", scu: 2750, pricePerScu: 27.5, cargoSizes: [] },
        { name: "Processed Food", status: "low", scu: 100, pricePerScu: 10, cargoSizes: [] },
      ],
    }),
  },
];

export const mockBackendState = {
  appConfig: mockAppConfig,
  account: mockAccount,
  ocrStatus: mockOcrStatus,
  terminals: mockTerminals,
  commodities: mockCommodities,
  dataParameters: mockDataParameters,
  pendingScreenshots: mockPendingScreenshots,
  processedScreenshots: mockProcessedScreenshots,
};

export function mockBackendResponse<T>(command: string, args?: Record<string, unknown>): T {
  switch (command) {
    case "load_config":
      return mockAppConfig as T;
    case "save_config":
      return { ...mockAppConfig, ...(args?.config as Partial<AppConfig> | undefined) } as T;
    case "list_screenshots":
      return mockPendingScreenshots as T;
    case "prefetch_terminals":
      return { gameVersion: "4.2.1", terminals: mockTerminals, source: "mock" } as T;
    case "prefetch_commodities":
      return { gameVersion: "4.2.1", commodities: mockCommodities, source: "mock" } as T;
    case "prefetch_data_parameters":
      return { gameVersion: "4.2.1", parameters: mockDataParameters, source: "mock" } as T;
    case "process_screenshots":
    case "process_selected_screenshots":
      return { screenshots: mockProcessedScreenshots, warnings: [] } as T;
    case "search_commodities":
      return mockCommodities as T;
    case "check_uex_account":
      return mockAccount as T;
    case "get_ocr_status":
      return mockOcrStatus as T;
    case "submit_to_uex":
      return { status: "ok" } as T;
    case "delete_submitted_screenshots":
      return mockAppConfig as T;
    case "load_working_set":
      return null as T;
    case "save_working_set":
    case "clear_working_set":
    case "hide_to_tray":
    case "cancel_ocr":
    case "open_screenshot":
      return undefined as T;
    default:
      throw new Error(`No mock backend response for command: ${command}`);
  }
}
