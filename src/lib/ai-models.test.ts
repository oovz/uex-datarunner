import { describe, expect, it } from "vitest";
import { AI_MODEL_OPTIONS, QWEN3_VL_4B_INSTRUCT_CUDA_ALIAS, describeAiModel } from "./ai-models";

describe("AI model options", () => {
  it("offers only NVIDIA CUDA Foundry Local model options", () => {
    expect(AI_MODEL_OPTIONS.map((option) => option.value)).toContain(
      QWEN3_VL_4B_INSTRUCT_CUDA_ALIAS,
    );
    expect(AI_MODEL_OPTIONS.every((option) => option.runtime === "NVIDIA CUDA")).toBe(true);
    expect(JSON.stringify(AI_MODEL_OPTIONS)).not.toMatch(/DirectML|Windows ML/i);
  });

  it("falls back to the primary CUDA model for unknown persisted values", () => {
    expect(describeAiModel("qwen3-vl-4b-instruct").runtime).toBe("NVIDIA CUDA");
  });
});
