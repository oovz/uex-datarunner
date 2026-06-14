import type { AppConfig } from "./types";

export type UexAccountCheck = {
  canSubmit: boolean;
  label: string | null;
  reason: string | null;
};

export type OcrStatus = {
  isReady: boolean;
  source: "foundryLocalCuda" | "missing";
  path: string | null;
  message: string;
  gpuName: string | null;
  gpuVendor: string;
  isModelLoaded?: boolean;
  loadedModelId?: string | null;
  selectedModelId?: string | null;
};

export function getSetupState({
  config,
  account,
  ocr,
}: {
  config: AppConfig;
  account: UexAccountCheck | null;
  ocr: OcrStatus | null;
}) {
  const missing: string[] = [];

  if (!config.screenshotDir.trim()) {
    missing.push("screenshot folder");
  }
  if (!config.secretKey.trim()) {
    missing.push("UEX secret key");
  }
  if (!account?.canSubmit) {
    missing.push("eligible UEX datarunner account");
  }
  if (!ocr?.isReady) {
    missing.push("Foundry Local GPU OCR model");
  }

  return {
    isComplete: missing.length === 0,
    missing,
  };
}
