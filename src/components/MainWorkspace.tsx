import type { Dispatch, SetStateAction } from "react";
import {
  AlertTriangle,
  Camera,
  ClipboardCheck,
  Cpu,
  Eye,
  FilePlus2,
  Loader2,
  RefreshCcw,
  Search,
  Send,
  Settings,
  ShieldCheck,
  Trash2,
  XCircle,
} from "lucide-react";
import type {
  AppConfig,
  CommodityRow,
  OcrScreenshotState,
  ScreenshotFile,
  UexCommodity,
  UexTerminal,
} from "../lib/types";
import type { OcrStatus } from "../lib/setup";
import {
  formatTerminalLabel,
  getRowMarketSide,
  getRowMarketValues,
  patchRowMarketValues,
  type MarketSide,
} from "../lib/uex";
import { Badge, Button, Field, Input, Select } from "./ui";

export interface MainWorkspaceProps {
  commodities: UexCommodity[];
  config: AppConfig;
  isBusy: boolean;
  locationFilters: { system: string; planet: string };
  loadCommodities: (query: string) => Promise<UexCommodity[]>;
  pendingScreenshots: ScreenshotFile[];
  ocrScreenshotStates: Record<string, OcrScreenshotState>;
  processSelectedScreenshots: () => Promise<void>;
  refreshPending: () => Promise<void>;
  openScreenshot: (path: string) => Promise<void>;
  refreshTerminalCache: () => Promise<UexTerminal[]>;
  loadCommodityCache: () => Promise<UexCommodity[]>;
  rowFilter: string;
  rows: CommodityRow[];
  selectedScreenshotPaths: Set<string>;
  selectedTerminal: UexTerminal | null;
  selectAllScreenshots: (select: boolean) => void;
  setConfig: Dispatch<SetStateAction<AppConfig>>;
  setLocationFilters: Dispatch<SetStateAction<{ system: string; planet: string }>>;
  setRowFilter: (filter: string) => void;
  setTerminalId: (id: number | null) => void;
  setTerminalQuery: (query: string) => void;
  showSettings: () => void;
  submit: () => Promise<void>;
  submitReady: boolean;
  terminalId: number | null;
  terminalQuery: string;
  terminals: UexTerminal[];
  toggleScreenshotSelection: (path: string) => void;
  updateRow: (rowId: string, patch: Partial<CommodityRow>) => void;
  removeRow: (rowId: string) => void;
  ocr: OcrStatus | null;
  clearAll: () => void;
  cancelOcr: () => Promise<void>;
  ocrProgress: string;
  status: string;
}

const STATUS_OPTIONS = [
  { value: "1", label: "Out of stock" },
  { value: "2", label: "Very low" },
  { value: "3", label: "Low" },
  { value: "4", label: "Medium" },
  { value: "5", label: "High" },
  { value: "6", label: "Very high" },
  { value: "7", label: "Maximum" },
];

const CARGO_SIZES = [1, 2, 4, 8, 16, 24, 32];

export function MainWorkspace(props: MainWorkspaceProps) {
  const {
    commodities,
    config,
    isBusy,
    locationFilters,
    loadCommodities,
    pendingScreenshots,
    ocrScreenshotStates,
    processSelectedScreenshots,
    refreshPending,
    openScreenshot,
    refreshTerminalCache,
    loadCommodityCache,
    rowFilter,
    rows,
    selectedScreenshotPaths,
    selectedTerminal,
    selectAllScreenshots,
    setConfig,
    setLocationFilters,
    setRowFilter,
    setTerminalId,
    setTerminalQuery,
    showSettings,
    submit,
    submitReady,
    terminalId,
    terminalQuery,
    terminals,
    toggleScreenshotSelection,
    updateRow,
    removeRow,
    ocr,
    clearAll,
    cancelOcr,
    ocrProgress,
    status,
  } = props;

  const systems = uniqueCompact(terminals.map((t) => t.star_system_name));
  const planets = uniqueCompact(
    terminals
      .filter((t) => !locationFilters.system || t.star_system_name === locationFilters.system)
      .map((t) => t.planet_name ?? t.moon_name ?? t.orbit_name ?? t.space_station_name),
  );
  const visibleTerminals = terminals.filter((t) => {
    const body = t.planet_name ?? t.moon_name ?? t.orbit_name ?? t.space_station_name ?? "";
    const haystack = [
      t.name,
      t.fullname,
      t.displayname,
      t.code,
      t.star_system_name,
      t.planet_name,
      t.moon_name,
      t.space_station_name,
      t.outpost_name,
      t.city_name,
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return (
      (!locationFilters.system || t.star_system_name === locationFilters.system) &&
      (!locationFilters.planet || body === locationFilters.planet) &&
      (!terminalQuery.trim() || haystack.includes(terminalQuery.trim().toLowerCase()))
    );
  });
  const terminalOptions = visibleTerminals.slice(0, 60);
  const terminalSuggestions = terminalQuery.trim() ? terminalOptions.slice(0, 6) : [];
  const visibleRows = rows.filter((row) =>
    row.commodityName.toLowerCase().includes(rowFilter.trim().toLowerCase()),
  );

  const runtimeReady = ocr?.isReady ?? false;
  const runtimeTone = runtimeReady
    ? "success"
    : ocr?.gpuVendor?.includes("Missing")
      ? "warning"
      : "danger";
  const runtimeLabel = ocr?.gpuVendor?.includes("CUDA")
    ? "CUDA"
    : ocr?.gpuVendor?.includes("CPU-Only")
      ? "Blocked"
      : "Unsupported GPU";
  const modelLabel = ocr?.loadedModelId ?? ocr?.selectedModelId ?? config.aiModel;
  const marketSides = rows.reduce(
    (counts, row) => {
      const side = getRowMarketSide(row);
      if (side) counts[side] += 1;
      return counts;
    },
    { buy: 0, sell: 0 },
  );
  const marketSide =
    marketSides.buy > 0 && marketSides.sell > 0
      ? `BUY ${marketSides.buy} / SELL ${marketSides.sell}`
      : String(rows.map(getRowMarketSide).find(Boolean) ?? "-").toUpperCase();
  const activity = ocrProgress || status;

  return (
    <section className="flex h-full min-h-0 flex-col bg-background text-foreground">
      <div className="flex items-center justify-between gap-3 border-b border-border bg-muted/40 px-3 py-1.5">
        <div className="flex min-w-0 items-center gap-2">
          <Cpu size={13} className={runtimeReady ? "text-sky-600" : "text-muted-foreground"} />
          <Badge tone={runtimeTone}>{runtimeReady ? "Ready" : "Check runtime"}</Badge>
          <span
            className="truncate text-[11px] text-muted-foreground"
            title={ocr?.gpuName ?? ocr?.gpuVendor ?? "Unknown GPU"}
          >
            {runtimeLabel} - {modelLabel}
          </span>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <span className="hidden max-w-[260px] truncate text-[11px] text-muted-foreground sm:inline-block">
            {activity}
          </span>
          <Button variant="ghost" onClick={showSettings} disabled={isBusy}>
            <Settings size={14} />
            Settings
          </Button>
        </div>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-[270px_1fr]">
        <aside
          className="flex min-h-0 flex-col border-r border-border"
          aria-label="Screenshot intake"
        >
          <header className="flex items-center gap-2 px-3 py-2">
            <h1 className="text-xs font-semibold uppercase tracking-wider text-foreground">
              Screenshots
            </h1>
            <span className="rounded-full bg-muted px-1.5 text-[10px] font-semibold text-muted-foreground">
              {pendingScreenshots.length}
            </span>
            <Button
              variant="ghost"
              className="ml-auto size-7 px-0"
              onClick={refreshPending}
              disabled={isBusy}
              aria-label="Refresh screenshots"
            >
              <RefreshCcw size={14} />
            </Button>
          </header>

          <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
            {pendingScreenshots.length === 0 ? (
              <div className="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground">
                <Camera size={24} />
                <span className="text-xs">No screenshots found</span>
              </div>
            ) : (
              <ul className="flex flex-col gap-1">
                {pendingScreenshots.map((file) => {
                  const selected = selectedScreenshotPaths.has(file.path);
                  const ocrState = ocrScreenshotStates[file.path];
                  const statusTone =
                    ocrState?.status === "failed"
                      ? "bg-red-100 text-red-700"
                      : ocrState?.status === "done"
                        ? "bg-emerald-100 text-emerald-700"
                        : ocrState?.status === "processing"
                          ? "bg-sky-100 text-sky-700"
                          : ocrState?.status === "queued"
                            ? "bg-muted text-muted-foreground"
                            : "";
                  const statusLabel = ocrState ? statusText(ocrState.status) : null;
                  return (
                    <li key={file.path}>
                      <div
                        className={
                          "flex items-center gap-2 rounded-md border px-2 py-1.5 transition-colors " +
                          (selected
                            ? "border-sky-400 bg-sky-50"
                            : "border-border bg-card hover:bg-accent")
                        }
                      >
                        <label className="flex min-w-0 flex-1 cursor-pointer items-center gap-2">
                          <input
                            type="checkbox"
                            className="accent-primary"
                            checked={selected}
                            onChange={() => toggleScreenshotSelection(file.path)}
                          />
                          <span className="min-w-0">
                            <span className="flex items-center gap-1.5">
                              <span className="block min-w-0 truncate text-xs text-foreground">
                                {file.filename}
                              </span>
                              {statusLabel ? (
                                <span
                                  className={`shrink-0 rounded px-1 text-[9px] font-semibold ${statusTone}`}
                                >
                                  {statusLabel}
                                </span>
                              ) : null}
                            </span>
                            <span
                              className="block truncate text-[10px] text-muted-foreground"
                              title={ocrState?.message}
                            >
                              {formatScreenshotTime(file.modifiedAtMs)}
                              {ocrState?.status === "failed" ? ` · ${ocrState.message}` : ""}
                            </span>
                          </span>
                        </label>
                        <Button
                          variant="ghost"
                          className="size-7 px-0"
                          onClick={() => void openScreenshot(file.path)}
                          aria-label={"Open " + file.filename}
                          title="Open screenshot"
                        >
                          <Eye size={13} />
                        </Button>
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>

          <footer className="flex flex-col gap-2 border-t border-border px-3 py-2">
            <div className="flex items-center justify-between text-[10px] text-muted-foreground">
              <span>{selectedScreenshotPaths.size} selected</span>
              <span>{pendingScreenshots.length} total</span>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {isBusy ? (
                <Button variant="danger" className="flex-1" onClick={cancelOcr}>
                  <XCircle size={14} />
                  Cancel
                </Button>
              ) : (
                <Button
                  className="flex-1"
                  onClick={processSelectedScreenshots}
                  disabled={selectedScreenshotPaths.size === 0}
                >
                  <FilePlus2 size={14} />
                  OCR Selected
                </Button>
              )}
              <Button
                variant="secondary"
                onClick={() => selectAllScreenshots(true)}
                disabled={pendingScreenshots.length === 0}
              >
                All
              </Button>
              <Button
                variant="ghost"
                onClick={() => selectAllScreenshots(false)}
                disabled={selectedScreenshotPaths.size === 0}
              >
                None
              </Button>
            </div>
          </footer>
        </aside>

        <main className="flex min-h-0 flex-col" aria-label="Review and submit commodity data">
          <section className="flex flex-col gap-2 border-b border-border px-3 py-2">
            <div className="grid grid-cols-2 gap-2">
              <Field label="System">
                <Select
                  value={locationFilters.system}
                  onChange={(e) => setLocationFilters({ system: e.target.value, planet: "" })}
                  disabled={systems.length === 0}
                >
                  <option value="">Any system</option>
                  {systems.map((s) => (
                    <option key={s} value={s}>
                      {s}
                    </option>
                  ))}
                </Select>
              </Field>
              <Field label="Planet / Moon">
                <Select
                  value={locationFilters.planet}
                  onChange={(e) => setLocationFilters((c) => ({ ...c, planet: e.target.value }))}
                  disabled={planets.length === 0}
                >
                  <option value="">Any body</option>
                  {planets.map((p) => (
                    <option key={p} value={p}>
                      {p}
                    </option>
                  ))}
                </Select>
              </Field>
            </div>
            <Field label="Find terminal">
              <div className="flex flex-col gap-1.5">
                <div className="flex gap-1.5">
                  <div className="relative flex-1">
                    <Search
                      size={13}
                      className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground"
                      aria-hidden="true"
                    />
                    <Input
                      name="terminalSearch"
                      autoComplete="off"
                      className="pl-7"
                      value={terminalQuery}
                      onChange={(e) => setTerminalQuery(e.target.value)}
                      placeholder="Terminal, city, outpost..."
                      aria-controls="terminal-suggestions"
                    />
                  </div>
                  <Button
                    variant="secondary"
                    onClick={() => void refreshTerminalCache()}
                    disabled={isBusy}
                    aria-label="Refresh terminal cache"
                  >
                    <RefreshCcw size={13} />
                  </Button>
                  <Button
                    variant="secondary"
                    onClick={() => void loadCommodityCache()}
                    disabled={isBusy}
                    aria-label="Refresh commodity cache"
                  >
                    <ClipboardCheck size={13} />
                  </Button>
                </div>
                {terminalSuggestions.length > 0 ? (
                  <div id="terminal-suggestions" className="grid grid-cols-2 gap-1" role="listbox">
                    {terminalSuggestions.map((terminal) => (
                      <button
                        key={terminal.id}
                        type="button"
                        className="truncate rounded-md border border-border bg-card px-2 py-1 text-left text-[11px] text-foreground hover:border-sky-300 hover:bg-sky-50"
                        onClick={() => {
                          setTerminalId(terminal.id);
                          setTerminalQuery(terminal.name);
                        }}
                        role="option"
                        aria-selected={terminal.id === terminalId}
                        title={formatTerminalLabel(terminal)}
                      >
                        {formatTerminalLabel(terminal)}
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
            </Field>
            <Field label="Terminal">
              <Select
                value={terminalId ?? ""}
                onChange={(e) => setTerminalId(e.target.value ? Number(e.target.value) : null)}
                disabled={terminalOptions.length === 0}
              >
                <option value="">Choose a terminal</option>
                {terminalOptions.map((t) => (
                  <option key={t.id} value={t.id}>
                    {formatTerminalLabel(t)}
                  </option>
                ))}
              </Select>
            </Field>
            <div className="flex items-center justify-between gap-2 text-[11px]">
              <span className="text-muted-foreground">
                Submission groups: <strong className="text-foreground">{marketSide}</strong>
              </span>
              <span className="terminal-card truncate rounded-md border border-border bg-muted/40 px-2 py-1 text-foreground">
                {selectedTerminal ? formatTerminalLabel(selectedTerminal) : "No terminal selected"}
              </span>
            </div>
          </section>

          <div className="flex items-center gap-2 px-3 py-2">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-foreground">
              Review Commodities
            </h2>
            <span className="rounded-full bg-sky-100 px-1.5 text-[10px] font-semibold text-sky-700">
              {rows.length} rows
            </span>
            {marketSides.buy > 0 ? (
              <span className="rounded-full bg-indigo-100 px-1.5 text-[10px] font-semibold text-indigo-700">
                {marketSides.buy} buy
              </span>
            ) : null}
            {marketSides.sell > 0 ? (
              <span className="rounded-full bg-emerald-100 px-1.5 text-[10px] font-semibold text-emerald-700">
                {marketSides.sell} sell
              </span>
            ) : null}
            <Input
              name="commodityFilter"
              autoComplete="off"
              className="ml-auto h-7 w-36"
              value={rowFilter}
              onChange={(e) => setRowFilter(e.target.value)}
              placeholder="Filter..."
            />
            <Button
              variant="ghost"
              onClick={clearAll}
              disabled={rows.length === 0}
              aria-label="Clear all rows"
            >
              <Trash2 size={13} />
            </Button>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto px-3 pb-2">
            {rows.length === 0 ? (
              <div className="flex h-full flex-col items-center justify-center gap-2 text-center text-muted-foreground">
                <ClipboardCheck size={28} />
                <span className="text-xs font-medium text-foreground">No commodity rows yet</span>
                <span className="max-w-[260px] text-[11px]">
                  Select screenshots on the left and run OCR to extract commodity rows for review.
                </span>
              </div>
            ) : (
              <ul className="flex flex-col gap-2.5">
                {visibleRows.map((row) => {
                  const side = getRowMarketSide(row) ?? "sell";
                  const values = getRowMarketValues(row, side);
                  const mapped = row.idCommodity !== null;
                  const sideTone =
                    side === "buy"
                      ? "border-l-indigo-500 bg-indigo-50/40"
                      : "border-l-emerald-500 bg-emerald-50/40";
                  return (
                    <li
                      key={row.id}
                      className={
                        "rounded-lg border border-border border-l-4 bg-card p-3 shadow-sm transition-colors hover:border-sky-300 " +
                        sideTone
                      }
                    >
                      <div className="flex items-center gap-2">
                        <span
                          className={
                            "shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase " +
                            (side === "buy"
                              ? "bg-indigo-600 text-white"
                              : "bg-emerald-600 text-white")
                          }
                        >
                          {side}
                        </span>
                        <Input
                          aria-label={"Commodity name for " + (row.commodityName || "row")}
                          className="flex-1"
                          list="commodity-options"
                          autoComplete="off"
                          value={row.commodityName}
                          onBlur={() => void loadCommodities(row.commodityName)}
                          onChange={(e) => updateRow(row.id, { commodityName: e.target.value })}
                        />
                        <span
                          className={
                            "shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium " +
                            (mapped
                              ? "bg-emerald-100 text-emerald-700"
                              : "bg-amber-100 text-amber-700")
                          }
                          title={
                            mapped
                              ? "Matched to UEX commodity " + row.idCommodity
                              : "Not matched in UEX cache"
                          }
                        >
                          {mapped ? "#" + row.idCommodity : "unmatched"}
                        </span>
                        <Button
                          variant="ghost"
                          className="size-7 px-0"
                          onClick={() => removeRow(row.id)}
                          aria-label={"Remove " + row.commodityName}
                        >
                          <Trash2 size={13} />
                        </Button>
                      </div>
                      <div className="mt-2 grid grid-cols-[5rem_1fr_1fr_1fr] gap-2">
                        <Field label="Side">
                          <Select
                            aria-label={"Market side for " + row.commodityName}
                            value={side}
                            onChange={(e) => {
                              const next = e.target.value as MarketSide;
                              updateRow(
                                row.id,
                                patchRowMarketValues(row, next, getRowMarketValues(row, side)),
                              );
                            }}
                          >
                            <option value="buy">Buy</option>
                            <option value="sell">Sell</option>
                          </Select>
                        </Field>
                        <Field label="Price / SCU">
                          <Input
                            type="number"
                            step="0.01"
                            aria-label={"Price for " + row.commodityName}
                            value={values.price ?? ""}
                            onChange={(e) =>
                              updateRow(
                                row.id,
                                patchRowMarketValues(row, side, {
                                  price: e.target.value ? Number(e.target.value) : null,
                                }),
                              )
                            }
                          />
                        </Field>
                        <Field label="SCU">
                          <Input
                            type="number"
                            aria-label={"SCU for " + row.commodityName}
                            value={values.scu ?? ""}
                            onChange={(e) =>
                              updateRow(
                                row.id,
                                patchRowMarketValues(row, side, {
                                  scu: e.target.value ? Number(e.target.value) : null,
                                }),
                              )
                            }
                          />
                        </Field>
                        <Field label="Status">
                          <Select
                            aria-label={"Status for " + row.commodityName}
                            value={values.status ?? ""}
                            onChange={(e) =>
                              updateRow(
                                row.id,
                                patchRowMarketValues(row, side, {
                                  status: e.target.value ? Number(e.target.value) : null,
                                }),
                              )
                            }
                          >
                            <option value="">-</option>
                            {STATUS_OPTIONS.map((o) => (
                              <option key={o.value} value={o.value}>
                                {o.label}
                              </option>
                            ))}
                          </Select>
                        </Field>
                      </div>
                      <div className="mt-2">
                        <Field label="Cargo sizes (SCU)">
                          <div
                            className="flex flex-wrap gap-1.5"
                            role="group"
                            aria-label={"Cargo sizes for " + row.commodityName}
                          >
                            {CARGO_SIZES.map((size) => {
                              const selected = row.cargoSizes.includes(size);
                              return (
                                <button
                                  key={size}
                                  type="button"
                                  className={
                                    "h-7 min-w-9 rounded-md border px-2 text-xs font-semibold transition-colors " +
                                    (selected
                                      ? "border-sky-600 bg-sky-600 text-white"
                                      : "border-border bg-white text-muted-foreground hover:border-sky-300 hover:text-foreground")
                                  }
                                  onClick={() => {
                                    const next = selected
                                      ? row.cargoSizes.filter((value) => value !== size)
                                      : [...row.cargoSizes, size].sort(
                                          (left, right) => left - right,
                                        );
                                    updateRow(row.id, { cargoSizes: next });
                                  }}
                                  aria-pressed={selected}
                                  aria-label={`${size} SCU cargo size`}
                                >
                                  {size}
                                </button>
                              );
                            })}
                          </div>
                        </Field>
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
            <datalist id="commodity-options">
              {commodities.map((c) => (
                <option key={c.id} value={c.name}>
                  {"#" + c.id}
                </option>
              ))}
            </datalist>
          </div>

          <footer className="flex items-center gap-3 border-t border-border bg-muted/40 px-3 py-2">
            <label className="flex cursor-pointer items-center gap-1.5 text-[11px] text-muted-foreground">
              <input
                type="checkbox"
                className="accent-primary"
                checked={config.deleteAfterSubmit}
                onChange={(e) => setConfig({ ...config, deleteAfterSubmit: e.target.checked })}
              />
              Delete after submit
            </label>
            <span className="ml-auto flex items-center gap-1.5 text-[11px] text-muted-foreground">
              {submitReady ? (
                <ShieldCheck size={13} className="text-emerald-600" />
              ) : (
                <AlertTriangle size={13} className="text-amber-600" />
              )}
              {submitReady ? "Ready to submit" : "Resolve rows + terminal"}
            </span>
            <Button onClick={submit} disabled={isBusy || !submitReady}>
              {isBusy ? <Loader2 size={14} className="spin" /> : <Send size={14} />}
              Submit to UEX
            </Button>
          </footer>
        </main>
      </div>
    </section>
  );
}

function formatScreenshotTime(ms: number): string {
  return new Date(ms).toLocaleString([], {
    month: "numeric",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

function uniqueCompact(values: Array<string | null | undefined>): string[] {
  return Array.from(new Set(values.filter((v): v is string => Boolean(v?.trim())))).sort((a, b) =>
    a.localeCompare(b),
  );
}

function statusText(status: OcrScreenshotState["status"]): string {
  switch (status) {
    case "queued":
      return "Queued";
    case "processing":
      return "OCR";
    case "done":
      return "Done";
    case "failed":
      return "Failed";
  }
}
