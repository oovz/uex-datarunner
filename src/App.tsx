import { Minus, Square, X, Loader2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { SetupWizard } from "./components/SetupWizard";
import { MainWorkspace } from "./components/MainWorkspace";
import { CUDA_VISION_MODEL_ALIAS } from "./lib/ai-models";
import {
  callBackend,
  cancelOcr,
  listenToOcrProgress,
  openDirectoryDialog,
  minimizeWindow,
  toggleMaximizeWindow,
  closeWindow,
  loadWorkingSet,
  saveWorkingSet,
  clearWorkingSet,
  openScreenshot,
} from "./lib/backend";
import { mergeCommodityRows, resolveCommodityIdsFromCache } from "./lib/commodity-session";
import { parseCommodityOcrText } from "./lib/ocr-parser";
import { getSetupState, type OcrStatus } from "./lib/setup";
import { isProductionSubmissionMode } from "./lib/submission-mode";
import {
  buildScreenshotCommoditySubmissions,
  describeUexSubmitStatus,
  formatTerminalLabel,
  getRowMarketSide,
  getRowMarketValues,
  patchRowMarketValues,
} from "./lib/uex";
import type {
  AppConfig,
  CommodityRow,
  ProcessedScreenshot,
  ScreenshotFile,
  UexAccountCheck,
  UexCommodity,
  UexDataParameters,
  OcrProgressEvent,
  OcrScreenshotState,
  UexTerminal,
} from "./lib/types";

const defaultConfig: AppConfig = {
  screenshotDir: "",
  secretKey: "",
  deleteAfterSubmit: false,
  isProduction: true,
  dataType: "commodity",
  aiModel: CUDA_VISION_MODEL_ALIAS,
  keepModelLoaded: false,
};

type Screen = "settings" | "main";

type ProcessResult = {
  screenshots: ProcessedScreenshot[];
  warnings: string[];
};

type TerminalCachePayload = {
  gameVersion: string;
  terminals: UexTerminal[];
  source: string;
};

type CommodityCachePayload = {
  gameVersion: string;
  commodities: UexCommodity[];
  source: string;
};

type DataParametersCachePayload = {
  gameVersion: string;
  parameters: UexDataParameters;
  source: string;
};

export default function App() {
  const [config, setConfig] = useState<AppConfig>(defaultConfig);
  const [isReady, setIsReady] = useState(false);
  const [isBusy, setIsBusy] = useState(false);
  const [status, setStatus] = useState("Starting up…");
  const [error, setError] = useState<string | null>(null);
  const [ocrProgress, setOcrProgress] = useState<string>("");
  const [warnings, setWarnings] = useState<string[]>([]);
  const [ocrScreenshotStates, setOcrScreenshotStates] = useState<
    Record<string, OcrScreenshotState>
  >({});
  const [account, setAccount] = useState<UexAccountCheck | null>(null);
  const [ocr, setOcr] = useState<OcrStatus | null>(null);
  const [screenshots, setScreenshots] = useState<ProcessedScreenshot[]>([]);
  const [pendingScreenshots, setPendingScreenshots] = useState<ScreenshotFile[]>([]);
  const [selectedScreenshotPaths, setSelectedScreenshotPaths] = useState<Set<string>>(new Set());
  const [rows, setRows] = useState<CommodityRow[]>([]);
  const [gameVersion, setGameVersion] = useState<string>("");
  const [terminalQuery, setTerminalQuery] = useState("");
  const [terminals, setTerminals] = useState<UexTerminal[]>([]);
  const [terminalCache, setTerminalCache] = useState<{
    gameVersion: string;
    source: string;
  } | null>(null);
  const [terminalId, setTerminalId] = useState<number | null>(null);
  const [commodities, setCommodities] = useState<UexCommodity[]>([]);
  const [commodityCache, setCommodityCache] = useState<{
    gameVersion: string;
    source: string;
    count: number;
  } | null>(null);
  const [dataParameters, setDataParameters] = useState<UexDataParameters | null>(null);
  const [locationFilters, setLocationFilters] = useState({ system: "", planet: "" });
  const [rowFilter, setRowFilter] = useState("");
  const [screen, setScreen] = useState<Screen>("settings");

  const setupState = useMemo(() => getSetupState({ config, account, ocr }), [account, config, ocr]);
  const selectedTerminal = terminals.find((t) => t.id === terminalId) ?? null;

  const submitReady =
    setupState.isComplete &&
    rows.length > 0 &&
    terminalId !== null &&
    rows.every((row) => row.idCommodity !== null && getRowMarketSide(row) !== null);

  useEffect(() => {
    void loadInitialState();
  }, []);

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    listenToOcrProgress((event) => {
      handleOcrProgress(event);
    }).then((unlisten) => {
      cleanup = unlisten;
    });
    return () => {
      if (cleanup) cleanup();
    };
  }, []);

  const handleOcrProgress = (event: OcrProgressEvent | string) => {
    if (typeof event === "string") {
      setStatus(event);
      setOcrProgress(event);
      return;
    }

    const message = event.data.message;
    setStatus(message);
    setOcrProgress(message);

    if (event.event === "screenshotStarted") {
      setOcrScreenshotStates((current) => ({
        ...current,
        [event.data.path]: { status: "processing", message },
      }));
      return;
    }

    if (event.event === "screenshotSucceeded") {
      setOcrScreenshotStates((current) => ({
        ...current,
        [event.data.path]: { status: "done", message },
      }));
      return;
    }

    if (event.event === "screenshotFailed") {
      setOcrScreenshotStates((current) => ({
        ...current,
        [event.data.path]: { status: "failed", message: event.data.error },
      }));
      setWarnings((current) =>
        uniqueMessages([...current, `${event.data.filename}: ${event.data.error}`]),
      );
    }
  };

  // Durable session persistence (req #6): keep the user's extracted commodities
  // and processed screenshots across minimize, tray-hide, and restart until they
  // submit or clear. Debounced so rapid edits coalesce into one write.
  useEffect(() => {
    if (!isReady) {
      return;
    }
    const handle = setTimeout(() => {
      if (rows.length === 0 && screenshots.length === 0) {
        void clearWorkingSet();
        return;
      }
      void saveWorkingSet({ version: 1, rows, screenshots, terminalId, gameVersion });
    }, 400);
    return () => clearTimeout(handle);
  }, [isReady, rows, screenshots, terminalId, gameVersion]);

  const loadInitialState = async () => {
    try {
      const loaded = await callBackend<AppConfig>("load_config");
      setConfig(loaded);
      const currentOcr = await callBackend<OcrStatus>("get_ocr_status");
      setOcr(currentOcr);
      if (loaded.secretKey.trim()) {
        setAccount(await callBackend<UexAccountCheck>("check_uex_account"));
      }
      // Restore an in-progress review session (commodities + screenshots) before
      // marking ready, so the debounced persistence effect never overwrites it
      // with an empty snapshot on launch.
      const savedSession = await loadWorkingSet();
      const restoredRowCount = savedSession?.rows?.length ?? 0;
      if (savedSession) {
        if (Array.isArray(savedSession.rows)) setRows(savedSession.rows);
        if (Array.isArray(savedSession.screenshots)) setScreenshots(savedSession.screenshots);
        if (typeof savedSession.terminalId === "number") setTerminalId(savedSession.terminalId);
        if (savedSession.gameVersion?.trim()) setGameVersion(savedSession.gameVersion);
      }
      setIsReady(true);
      const setupComplete = Boolean(loaded.screenshotDir.trim() && loaded.secretKey.trim());
      setScreen(setupComplete ? "main" : "settings");
      if (restoredRowCount > 0) {
        setStatus(
          `Restored ${restoredRowCount} unsubmitted commodity row(s) from your last session.`,
        );
      } else if (setupComplete) {
        setStatus("Ready. Select screenshots, then run OCR.");
      } else {
        setStatus("Add your screenshot folder and UEX secret key in Settings to begin.");
      }
      if (loaded.screenshotDir.trim()) {
        await refreshPending();
      }
      void loadDataParameters(false);
      void loadTerminalCache(false);
      void loadCommodityCache(false);
    } catch (cause) {
      setError(toMessage(cause));
      setIsReady(true);
    }
  };

  const refreshPending = async () => {
    try {
      setPendingScreenshots(await callBackend<ScreenshotFile[]>("list_screenshots"));
    } catch (cause) {
      setError(toMessage(cause));
    }
  };

  const saveSettings = async () => {
    setIsBusy(true);
    setError(null);
    try {
      const saved = await callBackend<AppConfig>("save_config", { config });
      setConfig(saved);
      setOcr(await callBackend<OcrStatus>("get_ocr_status"));
      if (saved.secretKey.trim()) {
        setAccount(await callBackend<UexAccountCheck>("check_uex_account"));
      }
      setStatus("Setup details verified.");
      await refreshPending();
      if (saved.screenshotDir.trim() && saved.secretKey.trim()) {
        setScreen("main");
      }
    } catch (cause) {
      setError(toMessage(cause));
    } finally {
      setIsBusy(false);
    }
  };

  const chooseDirectory = async () => {
    const selected = await openDirectoryDialog();
    if (selected) {
      setConfig((current) => ({ ...current, screenshotDir: selected }));
    }
  };

  const checkAccount = async () => {
    setIsBusy(true);
    setError(null);
    try {
      const saved = await callBackend<AppConfig>("save_config", { config });
      setConfig(saved);
      setAccount(await callBackend<UexAccountCheck>("check_uex_account"));
      setStatus("Credentials verification success.");
    } catch (cause) {
      setError(toMessage(cause));
    } finally {
      setIsBusy(false);
    }
  };

  const processSelectedScreenshots = async () => {
    if (selectedScreenshotPaths.size === 0) {
      setError("Select at least one screenshot to OCR.");
      return;
    }
    setIsBusy(true);
    setError(null);
    setWarnings([]);
    setOcrScreenshotStates((current) => {
      const next = { ...current };
      for (const path of selectedScreenshotPaths) {
        const file = pendingScreenshots.find((candidate) => candidate.path === path);
        next[path] = {
          status: "queued",
          message: file ? `Queued ${file.filename}` : "Queued for OCR",
        };
      }
      return next;
    });
    try {
      setStatus("Running optical analysis on screenshots...");
      const result = await callBackend<ProcessResult>("process_selected_screenshots", {
        paths: Array.from(selectedScreenshotPaths),
      });
      setScreenshots((current) => {
        const existingPaths = new Set(current.map((s) => s.file.path));
        const newScreenshots = result.screenshots.filter((s) => !existingPaths.has(s.file.path));
        return [...current, ...newScreenshots];
      });
      setWarnings((current) => uniqueMessages([...current, ...result.warnings]));
      const parsed = result.screenshots.flatMap((screenshot) =>
        parseCommodityOcrText(screenshot.ocrText, screenshot.file.path),
      );
      setSelectedScreenshotPaths(new Set());
      const mergedCount = await resolveCommodityIds(parsed);
      setStatus(
        result.screenshots.length === 0
          ? "No data extracted from selected screenshots."
          : `Processed ${result.screenshots.length} screenshot(s), extracted ${mergedCount} row(s).`,
      );
      await refreshPending();
    } catch (cause) {
      setError(toMessage(cause));
      setStatus("OCR did not complete.");
    } finally {
      setIsBusy(false);
    }
  };

  const toggleScreenshotSelection = (path: string) => {
    setSelectedScreenshotPaths((current) => {
      const next = new Set(current);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  const selectAllScreenshots = (select: boolean) => {
    if (select) {
      setSelectedScreenshotPaths(new Set(pendingScreenshots.map((file) => file.path)));
    } else {
      setSelectedScreenshotPaths(new Set());
    }
  };

  const resolveCommodityIds = async (parsedRows: CommodityRow[]) => {
    const cachedCommodities = await loadCommodityCache(false);
    const resolvedRows = resolveCommodityIdsFromCache(parsedRows, cachedCommodities);

    let mergedCount = 0;
    setRows((current) => {
      const merged = mergeCommodityRows(current, resolvedRows);
      mergedCount = merged.length;
      return merged;
    });
    return mergedCount || resolvedRows.length;
  };

  const loadCommodities = async (query: string) => {
    const matches = await callBackend<UexCommodity[]>("search_commodities", { query });
    setCommodities((existing) => mergeCommodities(existing, matches));
    return matches;
  };

  const loadCommodityCache = async (force: boolean) => {
    try {
      const payload = await callBackend<CommodityCachePayload>("prefetch_commodities", { force });
      setCommodities(payload.commodities);
      setCommodityCache({
        gameVersion: payload.gameVersion,
        source: payload.source,
        count: payload.commodities.length,
      });
      return payload.commodities;
    } catch (cause) {
      setError(toMessage(cause));
      return commodities;
    }
  };

  const loadTerminalCache = async (force: boolean) => {
    try {
      const payload = await callBackend<TerminalCachePayload>("prefetch_terminals", { force });
      setTerminals(payload.terminals);
      setTerminalCache({ gameVersion: payload.gameVersion, source: payload.source });
      if (payload.terminals.length > 0) {
        setTerminalId((current) => current ?? payload.terminals[0].id);
      }
      if (payload.gameVersion.trim()) {
        setGameVersion(payload.gameVersion);
      }
      return payload.terminals;
    } catch (cause) {
      setError(toMessage(cause));
      return [];
    }
  };

  const loadDataParameters = async (force: boolean) => {
    try {
      const payload = await callBackend<DataParametersCachePayload>("prefetch_data_parameters", {
        force,
      });
      setDataParameters(payload.parameters);
      if (payload.gameVersion.trim()) {
        setGameVersion(payload.gameVersion);
      }
    } catch (cause) {
      setError(toMessage(cause));
    }
  };

  const submit = async () => {
    if (!terminalId) {
      setError("Select a UEX terminal before submitting.");
      return;
    }

    setIsBusy(true);
    setError(null);
    try {
      setStatus("Submitting to UEX...");
      const submissions = buildScreenshotCommoditySubmissions({
        terminalId,
        isProduction: isProductionSubmissionMode(),
        gameVersion,
        screenshots,
        rows,
      });
      let submittedCount = 0;

      for (const submission of submissions) {
        const response = await callBackend<Record<string, unknown>>("submit_to_uex", {
          payload: submission.payload,
        });
        if (response.status !== "ok") {
          const status = String(response.status ?? "error");
          const screenshot = submission.screenshotPath
            ? screenshots.find((candidate) => candidate.file.path === submission.screenshotPath)
            : null;
          throw new Error(
            `${describeUexSubmitStatus(status)} (${screenshot?.file.filename ?? "manual rows"})`,
          );
        }
        submittedCount += 1;
      }

      const paths = submissions
        .map((submission) => submission.screenshotPath)
        .filter((path): path is string => path !== null);
      setConfig(await callBackend<AppConfig>("delete_submitted_screenshots", { paths }));
      setRows([]);
      setScreenshots([]);
      await clearWorkingSet();
      setStatus(`UEX accepted ${submittedCount} screenshot submission(s).`);
      await refreshPending();
    } catch (cause) {
      setError(toMessage(cause));
      setStatus("Submission failed.");
    } finally {
      setIsBusy(false);
    }
  };

  const updateRow = (rowId: string, patch: Partial<CommodityRow>) => {
    // If clearing completely, rows will be cleared (used inside clearAll)
    if (rowId === "") {
      return;
    }
    setRows((current) => current.map((row) => (row.id === rowId ? { ...row, ...patch } : row)));
  };

  const hideToTray = async () => {
    await callBackend("hide_to_tray");
  };

  if (!isReady) {
    return (
      <main className="flex h-screen items-center justify-center bg-background">
        <Loader2 className="spin text-muted-foreground" size={26} />
      </main>
    );
  }

  return (
    <main className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <header
        className="titlebar flex h-9 shrink-0 items-center justify-between border-b border-border bg-muted/40 pl-3"
        data-tauri-drag-region
      >
        <div
          className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-widest text-foreground"
          data-tauri-drag-region
        >
          <img src="/icon.svg" alt="" className="h-3.5 w-3.5" />
          <span>UEX Datarunner</span>
        </div>
        <div className="flex items-center">
          <button className="window-control-btn" aria-label="Minimize" onClick={minimizeWindow}>
            <Minus size={14} />
          </button>
          <button
            className="window-control-btn"
            aria-label="Maximize"
            onClick={toggleMaximizeWindow}
          >
            <Square size={10} />
          </button>
          <button className="window-control-btn close-btn" aria-label="Close" onClick={closeWindow}>
            <X size={14} />
          </button>
        </div>
      </header>

      <div className="relative flex min-h-0 flex-1 flex-col select-none">
        {error || warnings.length > 0 ? (
          <div className="pointer-events-none absolute right-2 top-2 z-10 flex w-[min(520px,calc(100%-1rem))] flex-col gap-1">
            {error ? (
              <div className="pointer-events-auto reveal flex items-start gap-2 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-xs font-medium text-red-700 shadow-sm">
                <span className="min-w-0 flex-1">{error}</span>
                <button
                  type="button"
                  className="shrink-0 rounded px-1 text-red-700 hover:bg-red-100"
                  onClick={() => setError(null)}
                  aria-label="Dismiss error"
                >
                  <X size={12} />
                </button>
              </div>
            ) : null}
            {warnings.map((warning) => (
              <div
                key={warning}
                className="pointer-events-auto reveal flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-xs font-medium text-amber-700 shadow-sm"
              >
                <span className="min-w-0 flex-1">{warning}</span>
                <button
                  type="button"
                  className="shrink-0 rounded px-1 text-amber-700 hover:bg-amber-100"
                  onClick={() =>
                    setWarnings((current) => current.filter((candidate) => candidate !== warning))
                  }
                  aria-label="Dismiss warning"
                >
                  <X size={12} />
                </button>
              </div>
            ))}
          </div>
        ) : null}

        {screen === "settings" || !setupState.isComplete ? (
          <SetupWizard
            account={account}
            checkAccount={checkAccount}
            chooseDirectory={chooseDirectory}
            config={config}
            isBusy={isBusy}
            ocr={ocr}
            saveSettings={saveSettings}
            setConfig={setConfig}
            setupState={setupState}
            onCancel={setupState.isComplete ? () => setScreen("main") : undefined}
          />
        ) : (
          <MainWorkspace
            commodities={commodities}
            config={config}
            isBusy={isBusy}
            locationFilters={locationFilters}
            loadCommodities={loadCommodities}
            pendingScreenshots={pendingScreenshots}
            ocrScreenshotStates={ocrScreenshotStates}
            processSelectedScreenshots={processSelectedScreenshots}
            refreshPending={refreshPending}
            openScreenshot={openScreenshot}
            refreshTerminalCache={() => loadTerminalCache(true)}
            loadCommodityCache={() => loadCommodityCache(false)}
            rowFilter={rowFilter}
            rows={rows}
            selectedScreenshotPaths={selectedScreenshotPaths}
            selectedTerminal={selectedTerminal}
            selectAllScreenshots={selectAllScreenshots}
            setConfig={setConfig}
            setLocationFilters={setLocationFilters}
            setRowFilter={setRowFilter}
            setTerminalId={setTerminalId}
            setTerminalQuery={setTerminalQuery}
            showSettings={() => setScreen("settings")}
            submit={submit}
            submitReady={submitReady}
            terminalId={terminalId}
            terminalQuery={terminalQuery}
            terminals={terminals}
            toggleScreenshotSelection={toggleScreenshotSelection}
            updateRow={updateRow}
            removeRow={(rowId) => setRows((current) => current.filter((r) => r.id !== rowId))}
            clearAll={() => {
              setRows([]);
              setScreenshots([]);
              void clearWorkingSet();
            }}
            ocr={ocr}
            cancelOcr={cancelOcr}
            ocrProgress={ocrProgress}
            status={status}
          />
        )}
      </div>
    </main>
  );
}

function toMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function mergeCommodities(existing: UexCommodity[], incoming: UexCommodity[]): UexCommodity[] {
  const byId = new Map<number, UexCommodity>();
  for (const commodity of [...existing, ...incoming]) {
    byId.set(commodity.id, commodity);
  }
  return Array.from(byId.values()).sort((a, b) => a.name.localeCompare(b.name));
}

function uniqueMessages(messages: string[]): string[] {
  return Array.from(new Set(messages.filter((message) => message.trim().length > 0)));
}
