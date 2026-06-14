/**
 * @vitest-environment jsdom
 */
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

const backendMock = vi.hoisted(() => ({
  progressListener: null as null | ((payload: unknown) => void),
  processSelectedResponse: null as null | Promise<unknown>,
}));

vi.mock("./lib/backend", async () => {
  const { mockAppConfig, mockBackendResponse } = await import("./mocks/backend");

  return {
    callBackend: vi.fn(async <T,>(command: string, args?: Record<string, unknown>) => {
      if (command === "process_selected_screenshots" && backendMock.processSelectedResponse) {
        return backendMock.processSelectedResponse as Promise<T>;
      }
      return mockBackendResponse<T>(command, args);
    }),
    openDirectoryDialog: vi.fn(async () => mockAppConfig.screenshotDir),
    listenToOcrProgress: vi.fn(async (callback: (payload: unknown) => void) => {
      backendMock.progressListener = callback;
      return () => {};
    }),
    cancelOcr: vi.fn(async () => {}),
    openScreenshot: vi.fn(async () => {}),
    minimizeWindow: vi.fn(),
    toggleMaximizeWindow: vi.fn(),
    closeWindow: vi.fn(),
    loadWorkingSet: vi.fn(async () => null),
    saveWorkingSet: vi.fn(async () => {}),
    clearWorkingSet: vi.fn(async () => {}),
  };
});

globalThis.IS_REACT_ACT_ENVIRONMENT = true;

let container: HTMLDivElement | null = null;
let root: Root | null = null;

afterEach(() => {
  if (root) {
    act(() => root?.unmount());
  }
  container?.remove();
  container = null;
  root = null;
  backendMock.progressListener = null;
  backendMock.processSelectedResponse = null;
});

describe("App", () => {
  it("renders the workflow with mock data without exposing UEX testing mode as a user choice", async () => {
    await renderApp();
    await waitForText("Screenshots");

    expect(container?.textContent).toContain("Area18 TDD");
    expect(container?.textContent).not.toContain("Submit mode");
    expect(container?.textContent).not.toContain("Testing");
    expect(container?.textContent).not.toContain("Production");
  });

  it("lets a configured CUDA user reopen settings with the Qwen 3.5 4B model selected", async () => {
    await renderApp();
    await waitForText("Screenshots");

    await clickButton("Settings");
    await waitForText("Settings");

    expect(container?.textContent).toContain("AI model");
    expect(
      (container?.querySelector('select[name="aiModel"]') as HTMLSelectElement | null)?.value,
    ).toBe("qwen3.5-4b");
  });

  it("filters terminal options locally while typing and does not show UEX IDs as terminal names", async () => {
    await renderApp();
    await waitForText("Screenshots");

    await typeInput('input[name="terminalSearch"]', "area");
    await waitForText("Area18 TDD · Area18");

    expect(container?.textContent).toContain("Area18 TDD · Area18");
    expect(container?.textContent).not.toContain("Area18 TDD #89");
  });

  it("reviews OCR rows with one editable market side instead of buy and sell column groups", async () => {
    await renderApp();
    await waitForText("Screenshots");

    await clickCheckbox();
    await clickButton("OCR Selected");
    await waitForText("2 rows");

    expect(inputValues()).toContain("Agricium");
    expect(container?.textContent).toContain("Side");
    expect(container?.textContent).toContain("Price / SCU");
    expect(container?.textContent).toContain("SCU");
    expect(container?.textContent).toContain("Status");
    expect(container?.textContent).not.toContain("UEX ID");
    expect(container?.textContent).not.toContain("Issues");
    expect(container?.textContent).not.toContain("Ready for UEX submission");
    expect(container?.textContent).not.toContain("ready for final review");
    expect(container?.textContent).not.toContain("Buy SCU");
    expect(container?.textContent).not.toContain("Sell SCU");
  });

  it("marks failed screenshots from OCR progress events before the whole OCR command finishes", async () => {
    backendMock.processSelectedResponse = new Promise(() => {});

    await renderApp();
    await waitForText("Screenshots");

    await clickCheckbox();
    await clickButton("OCR Selected");
    await waitForText("Running optical analysis");

    await act(async () => {
      backendMock.progressListener?.({
        event: "screenshotFailed",
        data: {
          path: "C:\\Mock\\StarCitizen\\screenshots\\ScreenShot-0001.jpg",
          filename: "ScreenShot-0001.jpg",
          error: "Foundry OCR request timed out after 90s.",
        },
      });
    });

    await waitForText("Failed");
    expect(container?.textContent).toContain("Foundry OCR request timed out after 90s.");
  });

  it("lets users dismiss OCR warnings so they do not keep blocking the workspace", async () => {
    backendMock.processSelectedResponse = Promise.resolve({
      screenshots: [],
      warnings: ["ScreenShot-0001.jpg: Foundry OCR request timed out after 90s."],
    });

    await renderApp();
    await waitForText("Screenshots");

    await clickCheckbox();
    await clickButton("OCR Selected");
    await waitForText("Foundry OCR request timed out after 90s.");

    await clickButton("Dismiss warning");
    expect(container?.textContent).not.toContain("Foundry OCR request timed out after 90s.");
  });
});

async function renderApp() {
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);

  await act(async () => {
    root?.render(<App />);
  });
}

async function waitForText(text: string) {
  for (let attempt = 0; attempt < 50; attempt += 1) {
    if (container?.textContent?.includes(text)) {
      return;
    }

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
  }

  throw new Error(`Timed out waiting for text: ${text}`);
}

async function clickButton(name: string) {
  const button = Array.from(container?.querySelectorAll("button") ?? []).find(
    (candidate) =>
      candidate.textContent?.includes(name) || candidate.getAttribute("aria-label") === name,
  );
  if (!button) {
    throw new Error(`Could not find button: ${name}`);
  }

  await act(async () => {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

async function clickCheckbox() {
  const checkbox = container?.querySelector('input[type="checkbox"]');
  if (!checkbox) {
    throw new Error("Could not find screenshot checkbox");
  }
  await act(async () => {
    checkbox.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

function inputValues(): string[] {
  return Array.from(container?.querySelectorAll("input") ?? [], (input) => input.value);
}

async function typeInput(selector: string, value: string) {
  const input = container?.querySelector(selector) as HTMLInputElement | null;
  if (!input) {
    throw new Error(`Could not find input: ${selector}`);
  }
  await act(async () => {
    input.value = value;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
