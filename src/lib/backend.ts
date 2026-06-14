import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AppConfig,
  OcrProgressEvent,
  ProcessedScreenshot,
  ScreenshotFile,
  UexAccountCheck,
  UexCommodity,
  UexDataParameters,
  WorkingSet,
} from "./types";
import type { OcrStatus } from "./setup";
import { isMockBackendEnabled } from "./submission-mode";
import { CUDA_VISION_MODEL_ALIAS } from "./ai-models";

export async function listenToOcrProgress(
  callback: (event: OcrProgressEvent | string) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }
  const unlisten = await listen<OcrProgressEvent | { message: string }>("ocr-progress", (event) => {
    const payload = event.payload;
    if (typeof payload === "object" && payload !== null && "event" in payload) {
      callback(payload as OcrProgressEvent);
      return;
    }
    callback({
      event: "message",
      data: { message: "message" in payload ? String(payload.message) : String(payload) },
    });
  });
  return unlisten;
}

type ProcessResult = {
  screenshots: ProcessedScreenshot[];
  warnings: string[];
};

type TerminalCachePayload<T> = {
  gameVersion: string;
  terminals: T[];
  source: string;
};

type CommodityCachePayload = {
  gameVersion: string;
  commodities: UexCommodity[];
  source: string;
};

type DataParametersCachePayload = {
  gameVersion: string;
  parameters: UexDataParameters;
  source: string;
};

type TestBackend = (command: string, args?: Record<string, unknown>) => unknown | Promise<unknown>;

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    __UEX_TEST_BACKEND__?: TestBackend;
  }
}

export async function callBackend<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) {
    return invoke<T>(command, args);
  }

  return browserPreviewResponse<T>(command, args);
}

export async function openDirectoryDialog(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select Star Citizen screenshots folder",
  });
  return typeof selected === "string" ? selected : null;
}

export async function openScreenshot(path: string): Promise<void> {
  await callBackend("open_screenshot", { path });
}

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined;
}

function browserPreviewResponse<T>(
  command: string,
  args?: Record<string, unknown>,
): T | Promise<T> {
  if (isMockBackendEnabled()) {
    const testBackend = window.__UEX_TEST_BACKEND__;
    if (!testBackend) {
      throw new Error("Mock backend mode is enabled, but no test backend has been installed.");
    }

    return testBackend(command, args) as T | Promise<T>;
  }

  switch (command) {
    case "load_config":
      return {
        screenshotDir: "",
        secretKey: "",
        deleteAfterSubmit: false,
        isProduction: true,
        dataType: "commodity",
        aiModel: CUDA_VISION_MODEL_ALIAS,
        keepModelLoaded: false,
      } satisfies AppConfig as T;
    case "save_config":
      return args?.config as T;
    case "list_screenshots":
    case "search_commodities":
      return [] as T;
    case "prefetch_terminals":
      return {
        gameVersion: "preview",
        terminals: [],
        source: "preview",
      } satisfies TerminalCachePayload<unknown> as T;
    case "prefetch_commodities":
      return {
        gameVersion: "preview",
        commodities: [],
        source: "preview",
      } satisfies CommodityCachePayload as T;
    case "prefetch_data_parameters":
      return {
        gameVersion: "preview",
        parameters: { is_accepting_reports: 1, game_version: "preview" },
        source: "preview",
      } satisfies DataParametersCachePayload as T;
    case "process_screenshots":
    case "process_selected_screenshots":
      return { screenshots: [], warnings: [] } satisfies ProcessResult as T;
    case "check_uex_account":
      return {
        canSubmit: false,
        label: null,
        reason: "Browser preview cannot verify UEX credentials.",
        rawStatus: "preview",
      } satisfies UexAccountCheck as T;
    case "get_ocr_status":
      return {
        isReady: true,
        source: "foundryLocalCuda",
        path: null,
        message: "Foundry Local OCR is assumed ready in browser preview.",
        gpuName: "NVIDIA GeForce RTX 4090",
        gpuVendor: "NVIDIA CUDA",
      } satisfies OcrStatus as T;
    case "submit_to_uex":
      return { status: "ok" } as T;
    case "delete_submitted_screenshots":
      return {
        screenshotDir: "",
        secretKey: "",
        deleteAfterSubmit: false,
        isProduction: true,
        dataType: "commodity",
        aiModel: CUDA_VISION_MODEL_ALIAS,
        keepModelLoaded: false,
      } satisfies AppConfig as T;
    case "load_working_set":
      return null as T;
    case "save_working_set":
    case "clear_working_set":
      return undefined as T;
    case "hide_to_tray":
    case "cancel_ocr":
    case "open_screenshot":
      return undefined as T;
    default:
      throw new Error(`No browser preview response for backend command: ${command}`);
  }
}

export async function cancelOcr(): Promise<void> {
  await callBackend("cancel_ocr");
}

export async function loadWorkingSet(): Promise<WorkingSet | null> {
  return callBackend<WorkingSet | null>("load_working_set");
}

export async function saveWorkingSet(snapshot: WorkingSet): Promise<void> {
  await callBackend("save_working_set", { snapshot });
}

export async function clearWorkingSet(): Promise<void> {
  await callBackend("clear_working_set");
}

export async function minimizeWindow(): Promise<void> {
  if (isTauriRuntime()) {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  }
}

export async function toggleMaximizeWindow(): Promise<void> {
  if (isTauriRuntime()) {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().toggleMaximize();
  }
}

export async function closeWindow(): Promise<void> {
  if (isTauriRuntime()) {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  }
}
