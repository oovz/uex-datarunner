import type { Page } from "@playwright/test";
import { mockBackendState } from "../../src/mocks/backend";

declare global {
  interface Window {
    __UEX_TEST_BACKEND__?: (command: string, args?: Record<string, unknown>) => unknown;
  }
}

export async function installMockBackend(page: Page): Promise<void> {
  await page.addInitScript((state) => {
    window.__UEX_TEST_BACKEND__ = (command, args) => {
      switch (command) {
        case "load_config":
          return state.appConfig;
        case "save_config":
          return { ...state.appConfig, ...(args?.config ?? {}) };
        case "list_screenshots":
          return state.pendingScreenshots;
        case "prefetch_terminals":
          return { gameVersion: "4.2.1", terminals: state.terminals, source: "mock" };
        case "prefetch_commodities":
          return { gameVersion: "4.2.1", commodities: state.commodities, source: "mock" };
        case "prefetch_data_parameters":
          return { gameVersion: "4.2.1", parameters: state.dataParameters, source: "mock" };
        case "process_screenshots":
        case "process_selected_screenshots":
          return { screenshots: state.processedScreenshots, warnings: [] };
        case "search_commodities":
          return state.commodities;
        case "check_uex_account":
          return state.account;
        case "get_ocr_status":
          return state.ocrStatus;
        case "submit_to_uex":
          return { status: "ok" };
        case "delete_submitted_screenshots":
          return state.appConfig;
        case "load_working_set":
          return null;
        case "save_working_set":
        case "clear_working_set":
        case "hide_to_tray":
        case "cancel_ocr":
        case "open_screenshot":
          return undefined;
        default:
          throw new Error(`No mock backend response for command: ${command}`);
      }
    };
  }, mockBackendState);
}
