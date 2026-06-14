import {
  AlertTriangle,
  CheckCircle2,
  Cpu,
  FolderOpen,
  KeyRound,
  Loader2,
  Save,
  ShieldAlert,
  ShieldCheck,
} from "lucide-react";
import type { Dispatch, ReactNode, SetStateAction } from "react";
import type { AppConfig, UexAccountCheck } from "../lib/types";
import type { OcrStatus } from "../lib/setup";
import { AI_MODEL_OPTIONS, CUDA_VISION_MODEL_ALIAS, describeAiModel } from "../lib/ai-models";
import { Badge, Button, Field, Input, Select } from "./ui";

export interface SetupWizardProps {
  account: UexAccountCheck | null;
  checkAccount: () => Promise<void>;
  chooseDirectory: () => Promise<void>;
  config: AppConfig;
  isBusy: boolean;
  ocr: OcrStatus | null;
  saveSettings: () => Promise<void>;
  setConfig: Dispatch<SetStateAction<AppConfig>>;
  setupState: { isComplete: boolean; missing: string[] };
  onCancel?: () => void;
}

export function SetupWizard({
  account,
  checkAccount,
  chooseDirectory,
  config,
  isBusy,
  ocr,
  saveSettings,
  setConfig,
  setupState,
  onCancel,
}: SetupWizardProps) {
  const selectedModel = describeAiModel(config.aiModel);
  const folderReady = Boolean(config.screenshotDir.trim());
  const accountReady = Boolean(config.secretKey.trim()) && account?.canSubmit === true;
  const runtimeReady = ocr?.isReady === true;
  const runtimeTone = runtimeReady
    ? "success"
    : ocr?.gpuVendor?.includes("Missing")
      ? "warning"
      : "danger";

  return (
    <section className="reveal flex h-full min-h-0 flex-col bg-background text-foreground">
      <header className="flex items-center justify-between border-b border-border px-4 py-2.5">
        <div>
          <h1 className="text-sm font-semibold text-foreground">Settings</h1>
          <p className="text-[11px] text-muted-foreground">
            {selectedModel.runtime} - {ocr?.selectedModelId ?? config.aiModel}
          </p>
        </div>
        {onCancel ? (
          <Button variant="ghost" onClick={onCancel}>
            Close
          </Button>
        ) : null}
      </header>

      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 py-3">
        <div className="flex flex-wrap gap-2">
          <StatusChip label="Screenshots" ready={folderReady} />
          <StatusChip label="UEX account" ready={accountReady} />
          <StatusChip label="GPU runtime" ready={runtimeReady} />
        </div>

        <Section icon={<FolderOpen size={15} />} title="Screenshot intake">
          <Field label="Screenshot folder">
            <div className="flex gap-1.5">
              <Input
                name="screenshotDir"
                autoComplete="off"
                value={config.screenshotDir}
                placeholder="C:\Users\you\Pictures\Star Citizen"
                onChange={(e) => setConfig({ ...config, screenshotDir: e.target.value })}
              />
              <Button
                variant="secondary"
                onClick={chooseDirectory}
                aria-label="Choose screenshot folder"
              >
                <FolderOpen size={14} />
                Browse
              </Button>
            </div>
          </Field>
        </Section>

        <Section
          icon={<KeyRound size={15} />}
          title="UEX datarunner account"
          badge={account?.canSubmit ? <Badge tone="success">Verified</Badge> : null}
        >
          <Field label="UEX secret key">
            <div className="flex gap-1.5">
              <Input
                name="secretKey"
                type="password"
                autoComplete="off"
                value={config.secretKey}
                placeholder="Enter secret key..."
                onChange={(e) => setConfig({ ...config, secretKey: e.target.value })}
              />
              <Button
                variant="secondary"
                onClick={checkAccount}
                disabled={isBusy || !config.secretKey.trim()}
              >
                {account?.canSubmit ? <ShieldCheck size={14} /> : <ShieldAlert size={14} />}
                Verify
              </Button>
            </div>
          </Field>
          {account?.reason ? <Notice tone="danger">{account.reason}</Notice> : null}
          {account?.canSubmit && account.label ? (
            <Notice tone="success">{"Verified as " + account.label}</Notice>
          ) : null}
        </Section>

        <Section
          icon={<Cpu size={15} />}
          title="Foundry Local runtime"
          badge={<Badge tone={runtimeTone}>{runtimeReady ? "Ready" : "Needs attention"}</Badge>}
        >
          <Field label="AI model">
            <Select
              name="aiModel"
              value={config.aiModel}
              onChange={(e) => setConfig({ ...config, aiModel: e.target.value })}
            >
              {AI_MODEL_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label + " - " + option.runtime}
                </option>
              ))}
            </Select>
          </Field>
          <p className="text-[11px] text-muted-foreground">
            {config.aiModel === CUDA_VISION_MODEL_ALIAS
              ? "Qwen 3.5 4B on Foundry Local + CUDA is the supported model."
              : "Qwen 3 VL 4B Instruct also requires Foundry Local + NVIDIA CUDA."}
          </p>

          <div className="grid grid-cols-2 gap-2">
            <Fact label="Runtime" value={selectedModel.runtime} />
            <Fact label="GPU" value={ocr?.gpuName ?? "Scanning local system"} />
            <Fact label="Provider" value={ocr?.gpuVendor ?? "Unknown"} />
            <Fact label="Model state" value={ocr?.isModelLoaded ? "Loaded" : "Not loaded"} />
          </div>

          <label className="flex cursor-pointer items-center gap-2 text-[11px] text-muted-foreground">
            <input
              type="checkbox"
              className="accent-primary"
              checked={config.keepModelLoaded}
              onChange={(e) => setConfig({ ...config, keepModelLoaded: e.target.checked })}
            />
            Keep the model loaded after OCR (faster repeat runs, more VRAM)
          </label>

          {ocr?.message ? (
            <Notice tone={runtimeReady ? "success" : "warning"}>{ocr.message}</Notice>
          ) : null}
        </Section>

        {setupState.missing.length > 0 ? (
          <p className="text-[11px] text-amber-600">
            {"Still required: " + setupState.missing.join(", ")}
          </p>
        ) : null}
      </div>

      <footer className="flex justify-end border-t border-border px-4 py-2.5">
        <Button
          onClick={saveSettings}
          disabled={isBusy || !folderReady || !accountReady || !runtimeReady}
        >
          {isBusy ? <Loader2 size={14} className="spin" /> : <Save size={14} />}
          Save Settings
        </Button>
      </footer>
    </section>
  );
}

function Section({
  icon,
  title,
  badge,
  children,
}: {
  icon: ReactNode;
  title: string;
  badge?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-3">
      <div className="flex items-center gap-2">
        <span className="text-sky-600">{icon}</span>
        <h2 className="text-xs font-semibold uppercase tracking-wider text-foreground">{title}</h2>
        {badge ? <span className="ml-auto">{badge}</span> : null}
      </div>
      {children}
    </section>
  );
}

function StatusChip({ label, ready }: { label: string; ready: boolean }) {
  return (
    <span
      className={
        "inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[11px] " +
        (ready
          ? "border-emerald-200 bg-emerald-50 text-emerald-700"
          : "border-amber-200 bg-amber-50 text-amber-700")
      }
    >
      {ready ? <CheckCircle2 size={12} /> : <AlertTriangle size={12} />}
      {label}
    </span>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-muted/40 px-2 py-1.5">
      <div className="text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
      <div className="truncate text-xs text-foreground" title={value}>
        {value}
      </div>
    </div>
  );
}

function Notice({
  tone,
  children,
}: {
  tone: "success" | "warning" | "danger";
  children: ReactNode;
}) {
  const toneClass =
    tone === "success"
      ? "border-emerald-200 bg-emerald-50 text-emerald-700"
      : tone === "warning"
        ? "border-amber-200 bg-amber-50 text-amber-700"
        : "border-red-200 bg-red-50 text-red-700";
  return (
    <p
      className={"flex items-start gap-1.5 rounded-md border px-2 py-1.5 text-[11px] " + toneClass}
    >
      {tone === "success" ? <ShieldCheck size={13} /> : <AlertTriangle size={13} />}
      <span>{children}</span>
    </p>
  );
}
