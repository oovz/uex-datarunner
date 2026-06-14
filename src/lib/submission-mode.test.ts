import { afterEach, describe, expect, it, vi } from "vitest";
import {
  getSubmissionMode,
  isMockBackendEnabled,
  isProductionSubmissionMode,
} from "./submission-mode";

describe("submission mode", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("uses production submissions by default", () => {
    expect(getSubmissionMode({})).toBe("production");
    expect(isProductionSubmissionMode({})).toBe(true);
  });

  it("uses UEX testing mode only when the test environment opts in", () => {
    expect(getSubmissionMode({ MODE: "test" })).toBe("testing");
    expect(getSubmissionMode({ VITE_UEX_SUBMIT_MODE: "testing" })).toBe("testing");
    expect(isProductionSubmissionMode({ VITE_UEX_SUBMIT_MODE: "testing" })).toBe(false);
  });

  it("enables the mock browser backend when the e2e fixture is installed", () => {
    expect(isMockBackendEnabled({})).toBe(false);

    vi.stubGlobal("window", { __UEX_TEST_BACKEND__: () => undefined });

    expect(isMockBackendEnabled({})).toBe(true);
  });
});
