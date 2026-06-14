import type { CommodityRow, UexCommodity } from "./types";

export function resolveCommodityIdsFromCache(
  rows: CommodityRow[],
  commodities: UexCommodity[],
): CommodityRow[] {
  const exactByName = new Map(
    commodities.map((commodity) => [normalizeName(commodity.name), commodity]),
  );
  const compactByName = uniqueByKey(commodities, (commodity) => compactName(commodity.name));

  return rows.map((row) => {
    if (row.idCommodity !== null) {
      return row;
    }

    const commodity =
      exactByName.get(normalizeName(row.commodityName)) ??
      compactByName.get(compactName(row.commodityName));
    return commodity === undefined
      ? row
      : { ...row, commodityName: commodity.name, idCommodity: commodity.id };
  });
}

export function mergeCommodityRows(
  currentRows: CommodityRow[],
  incomingRows: CommodityRow[],
): CommodityRow[] {
  const merged = [...currentRows];
  const indexByReviewKey = new Map(merged.map((row, index) => [reviewRowKey(row), index]));

  for (const incoming of incomingRows) {
    const key = reviewRowKey(incoming);
    const existingIndex = indexByReviewKey.get(key);
    if (existingIndex === undefined) {
      indexByReviewKey.set(key, merged.length);
      merged.push(incoming);
      continue;
    }

    merged[existingIndex] = mergeCommodityRow(merged[existingIndex], incoming);
  }

  return merged;
}

function mergeCommodityRow(existing: CommodityRow, incoming: CommodityRow): CommodityRow {
  return {
    ...existing,
    priceBuy: incoming.priceBuy !== null ? incoming.priceBuy : existing.priceBuy,
    scuBuy: incoming.scuBuy !== null ? incoming.scuBuy : existing.scuBuy,
    statusBuy: incoming.statusBuy !== null ? incoming.statusBuy : existing.statusBuy,
    priceSell: incoming.priceSell !== null ? incoming.priceSell : existing.priceSell,
    scuSell: incoming.scuSell !== null ? incoming.scuSell : existing.scuSell,
    statusSell: incoming.statusSell !== null ? incoming.statusSell : existing.statusSell,
    cargoSizes: Array.from(new Set([...existing.cargoSizes, ...incoming.cargoSizes])).sort(
      (left, right) => left - right,
    ),
    idCommodity: incoming.idCommodity !== null ? incoming.idCommodity : existing.idCommodity,
    screenshotPath: existing.screenshotPath ?? incoming.screenshotPath,
    sourceLines: incoming.sourceLines.length > 0 ? incoming.sourceLines : existing.sourceLines,
    confidence: Math.max(existing.confidence, incoming.confidence),
    issues: incoming.issues,
  };
}

function reviewRowKey(row: CommodityRow): string {
  return `${normalizeName(row.commodityName)}\0${rowMarketSide(row) ?? "unknown"}`;
}

function normalizeName(value: string): string {
  return value.trim().toLowerCase();
}

function compactName(value: string): string {
  return normalizeName(value).replace(/[^a-z0-9]/g, "");
}

function uniqueByKey(
  commodities: UexCommodity[],
  keyFor: (commodity: UexCommodity) => string,
): Map<string, UexCommodity> {
  const byKey = new Map<string, UexCommodity>();
  const duplicates = new Set<string>();

  for (const commodity of commodities) {
    const key = keyFor(commodity);
    if (!key) {
      continue;
    }
    if (byKey.has(key)) {
      duplicates.add(key);
      continue;
    }
    byKey.set(key, commodity);
  }

  for (const key of duplicates) {
    byKey.delete(key);
  }

  return byKey;
}

function rowMarketSide(row: CommodityRow): "buy" | "sell" | null {
  const hasBuy = row.priceBuy !== null || row.scuBuy !== null || row.statusBuy !== null;
  const hasSell = row.priceSell !== null || row.scuSell !== null || row.statusSell !== null;

  if (hasBuy && !hasSell) {
    return "buy";
  }
  if (hasSell && !hasBuy) {
    return "sell";
  }
  return null;
}
