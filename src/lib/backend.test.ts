/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { callBackend } from "./backend";

declare global {
  interface Window {
    __UEX_TEST_BACKEND__?: (command: string, args?: Record<string, unknown>) => unknown;
  }
}

afterEach(() => {
  delete window.__UEX_TEST_BACKEND__;
});

describe("callBackend", () => {
  it("uses an injected test backend instead of source-owned mock data when mock mode is enabled", async () => {
    const testBackend = vi.fn((command: string, args?: Record<string, unknown>) => ({
      command,
      args,
    }));
    window.__UEX_TEST_BACKEND__ = testBackend;

    await expect(callBackend("load_config", { source: "unit" })).resolves.toEqual({
      command: "load_config",
      args: { source: "unit" },
    });
    expect(testBackend).toHaveBeenCalledWith("load_config", { source: "unit" });
  });
});
