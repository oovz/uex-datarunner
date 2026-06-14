export type SubmissionMode = "production" | "testing";

type SubmissionEnv = Partial<{
  MODE: string;
  VITE_UEX_SUBMIT_MODE: string;
  VITE_UEX_MOCK_MODE: string;
}>;

const runtimeEnv = ((import.meta as ImportMeta & { env?: SubmissionEnv }).env ??
  {}) as SubmissionEnv;

export function getSubmissionMode(env: SubmissionEnv = runtimeEnv): SubmissionMode {
  if (env.MODE === "test" || env.VITE_UEX_SUBMIT_MODE === "testing") {
    return "testing";
  }

  return "production";
}

export function isProductionSubmissionMode(env?: SubmissionEnv): boolean {
  return getSubmissionMode(env) === "production";
}

export function isMockBackendEnabled(env: SubmissionEnv = runtimeEnv): boolean {
  if (env.MODE === "test" || env.VITE_UEX_MOCK_MODE === "workflow") {
    return true;
  }

  return (
    typeof window !== "undefined" &&
    typeof (window as Window & { __UEX_TEST_BACKEND__?: unknown }).__UEX_TEST_BACKEND__ ===
      "function"
  );
}
