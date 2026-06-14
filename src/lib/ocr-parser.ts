import type { CommodityRow } from "./types";

const STATUS_BY_LABEL = new Map<string, number>([
  ["out of stock", 1],
  ["empty", 1],
  ["库存已空", 1],
  ["库存极低", 2],
  ["库存偏少", 3],
  ["库存中等", 4],
  ["库存充足", 5],
  ["库存将满", 6],
  ["库存已满", 7],
  ["very low", 2],
  ["low", 3],
  ["medium", 4],
  ["high", 5],
  ["very high", 6],
  ["maximum", 7],
  ["full", 7],
]);

const ALLOWED_CARGO_SIZES = [1, 2, 4, 8, 16, 24, 32];

type MarketSide = "buy" | "sell";

type StructuredPayload = {
  marketSide: MarketSide | null;
  commodities: unknown[];
};

/**
 * Parses the structured JSON the OCR backend returns:
 *   { "marketSide": "buy"|"sell", "commodities": [ { name, status, scu, pricePerScu, cargoSizes } ] }
 *
 * Any model "thinking" (`<think>...</think>`) is stripped first, then the JSON
 * object is located by brace matching. We never guess from prose: if no
 * structured object is present, no rows are returned.
 */
export function parseCommodityOcrText(
  text: string,
  screenshotPath: string | null = null,
): CommodityRow[] {
  const cleaned = stripModelReasoning(text);
  for (const candidate of jsonObjectCandidates(cleaned)) {
    const payload = tryParsePayload(candidate);
    if (payload) {
      return parseStructuredPayload(payload, screenshotPath);
    }
  }
  return [];
}

/**
 * Removes model reasoning so only the structured answer remains. Handles paired
 * `<think>...</think>` blocks (case-insensitive), an unclosed opener, and an
 * orphan closing tag. Mirrors the backend's `strip_model_reasoning`.
 */
export function stripModelReasoning(text: string): string {
  const OPEN = "<think>";
  const CLOSE = "</think>";
  let output = "";
  let rest = text;

  for (;;) {
    const open = rest.toLowerCase().indexOf(OPEN);
    if (open === -1) {
      output += rest;
      break;
    }
    output += rest.slice(0, open);
    const afterOpen = rest.slice(open + OPEN.length);
    const close = afterOpen.toLowerCase().indexOf(CLOSE);
    if (close === -1) {
      // Unclosed reasoning block: drop the remainder.
      break;
    }
    rest = afterOpen.slice(close + CLOSE.length);
  }

  const orphan = output.toLowerCase().lastIndexOf(CLOSE);
  if (orphan !== -1) {
    output = output.slice(orphan + CLOSE.length);
  }

  return output.trim();
}

function tryParsePayload(candidate: string): StructuredPayload | null {
  let value: unknown;
  try {
    value = JSON.parse(candidate);
  } catch {
    return null;
  }
  if (!isRecord(value) || !Array.isArray(value.commodities)) {
    return null;
  }
  const marketSide =
    value.marketSide === "buy" || value.marketSide === "sell" ? value.marketSide : null;
  return { marketSide, commodities: value.commodities };
}

function parseStructuredPayload(
  payload: StructuredPayload,
  screenshotPath: string | null,
): CommodityRow[] {
  const documentSide = payload.marketSide;

  return payload.commodities.flatMap((entry) => {
    if (!isRecord(entry)) {
      return [];
    }

    const commodityName = stringField(entry.name);
    if (!commodityName) {
      return [];
    }

    const side: MarketSide | null =
      entry.marketSide === "buy" || entry.marketSide === "sell" ? entry.marketSide : documentSide;
    const price = numberField(entry.pricePerScu);
    const scu = numberField(entry.scu);
    const status =
      typeof entry.status === "string" ? detectStatus(entry.status) : numberField(entry.status);
    const cargoSizes = Array.isArray(entry.cargoSizes)
      ? Array.from(
          new Set(
            entry.cargoSizes
              .map(numberField)
              .filter(
                (value): value is number => value !== null && ALLOWED_CARGO_SIZES.includes(value),
              ),
          ),
        )
      : [];

    const issues: string[] = [];
    if (!side) {
      issues.push("Missing market side");
    }
    if (price === null) {
      issues.push("Missing price");
    }
    if (scu === null) {
      issues.push("Missing SCU");
    }
    if (status === null) {
      issues.push("Missing inventory status");
    }

    return [
      {
        id: stableRowId(),
        screenshotPath,
        commodityName,
        idCommodity: null,
        priceBuy: side === "buy" ? price : null,
        scuBuy: side === "buy" ? scu : null,
        statusBuy: side === "buy" ? status : null,
        priceSell: side === "sell" ? price : null,
        scuSell: side === "sell" ? scu : null,
        statusSell: side === "sell" ? status : null,
        cargoSizes,
        sourceLines: [JSON.stringify(entry)],
        confidence: roundConfidence(
          issues.length === 0 ? 0.92 : Math.max(0.42, 0.82 - issues.length * 0.16),
        ),
        issues,
      },
    ];
  });
}

/** Returns every brace-balanced `{...}` substring, in order, so we can locate the
 * JSON object even if the model wraps it in a stray prefix or suffix. */
function jsonObjectCandidates(text: string): string[] {
  const candidates: string[] = [];
  for (let index = 0; index < text.length; index += 1) {
    if (text[index] === "{") {
      const end = jsonObjectEnd(text.slice(index));
      if (end !== null) {
        candidates.push(text.slice(index, index + end));
      }
    }
  }
  return candidates;
}

function jsonObjectEnd(text: string): number | null {
  let depth = 0;
  let inString = false;
  let escaped = false;

  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (char === "\\") {
        escaped = true;
      } else if (char === '"') {
        inString = false;
      }
      continue;
    }

    if (char === '"') {
      inString = true;
    } else if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return index + 1;
      }
    }
  }

  return null;
}

/** Maps a status label (English or simplified/traditional Chinese, as emitted by
 * the model) to the UEX 1-7 inventory scale. Longest label first so "very low"
 * wins over "low". Clean enum strings only -- no fuzzy matching. */
function detectStatus(text: string): number | null {
  const lower = text.trim().toLowerCase();
  if (!lower) {
    return null;
  }
  const entries = Array.from(STATUS_BY_LABEL.entries()).sort((a, b) => b[0].length - a[0].length);
  return entries.find(([label]) => lower.includes(label))?.[1] ?? null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function stringField(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function numberField(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value !== "string" || !value.trim()) {
    return null;
  }

  const cleaned = value.replace(/,/g, "").trim();
  if (!/^-?\d+(\.\d+)?$/.test(cleaned)) {
    return null;
  }
  const numeric = Number(cleaned);
  return Number.isFinite(numeric) ? numeric : null;
}

function stableRowId(): string {
  return `ocr-${crypto.randomUUID()}`;
}

function roundConfidence(value: number): number {
  return Math.round(value * 100) / 100;
}
