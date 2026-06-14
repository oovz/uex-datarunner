import { describe, expect, it } from "vitest";
import { getSetupState } from "./setup";
import type { AppConfig } from "./types";

const baseConfig: AppConfig = {
  screenshotDir: "C:\\Shots",
  secretKey: "secret",
  deleteAfterSubmit: false,
  isProduction: false,
  dataType: "commodity",
  aiModel: "qwen-3-vl-4b-instruct",
  keepModelLoaded: false,
};

describe("getSetupState", () => {
  it("requires settings, UEX account eligibility, and OCR availability before main workflow", () => {
    expect(
      getSetupState({
        config: baseConfig,
        account: { canSubmit: true, label: "pilot", reason: null },
        ocr: { isReady: true, source: "foundryLocalCuda", path: null },
      }),
    ).toEqual({
      isComplete: true,
      missing: [],
    });

    expect(
      getSetupState({
        config: { ...baseConfig, secretKey: "" },
        account: { canSubmit: true, label: "pilot", reason: null },
        ocr: { isReady: true, source: "foundryLocalCuda", path: null },
      }),
    ).toEqual({
      isComplete: false,
      missing: ["UEX secret key"],
    });
  });

  it("describes the OCR requirement without hardcoding the CUDA-only model", () => {
    expect(
      getSetupState({
        config: baseConfig,
        account: { canSubmit: true, label: "pilot", reason: null },
        ocr: null,
      }),
    ).toEqual({
      isComplete: false,
      missing: ["Foundry Local GPU OCR model"],
    });
  });
});
