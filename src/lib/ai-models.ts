export const CUDA_VISION_MODEL_ALIAS = "qwen3.5-4b";
export const QWEN3_VL_4B_INSTRUCT_CUDA_ALIAS = "qwen-3-vl-4b-instruct";

export const AI_MODEL_OPTIONS = [
  {
    value: CUDA_VISION_MODEL_ALIAS,
    label: "Qwen 3.5 4B",
    runtime: "NVIDIA CUDA",
    description: "Use when an NVIDIA GPU and CUDA runtime are available.",
  },
  {
    value: QWEN3_VL_4B_INSTRUCT_CUDA_ALIAS,
    label: "Qwen 3 VL 4B Instruct",
    runtime: "NVIDIA CUDA",
    description: "Use the instruct vision variant when available in the local Foundry catalog.",
  },
] as const;

export function describeAiModel(value: string) {
  return AI_MODEL_OPTIONS.find((option) => option.value === value) ?? AI_MODEL_OPTIONS[0];
}
