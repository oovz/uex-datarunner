import { describe, expect, it } from "vitest";
import type { CommodityRow, UexCommodity } from "./types";
import { mergeCommodityRows, resolveCommodityIdsFromCache } from "./commodity-session";

describe("commodity review session rows", () => {
  it("merges the same commodity from repeated screenshots into one review row", () => {
    const current = [
      commodityRow({
        id: "row-a",
        screenshotPath: "shot-a.jpg",
        commodityName: "Agricium",
        priceSell: 27.5,
        scuSell: 100,
      }),
    ];
    const incoming = [
      commodityRow({
        id: "row-b",
        screenshotPath: "shot-b.jpg",
        commodityName: "Agricium",
        priceSell: 28,
        scuSell: 200,
      }),
    ];

    const merged = mergeCommodityRows(current, incoming);

    expect(merged).toHaveLength(1);
    expect(merged[0]).toEqual(
      expect.objectContaining({
        id: "row-a",
        screenshotPath: "shot-a.jpg",
        commodityName: "Agricium",
        priceSell: 28,
        scuSell: 200,
      }),
    );
  });

  it("preserves first screenshot ownership when a later screenshot repeats a commodity", () => {
    const rows = mergeCommodityRows(
      [
        commodityRow({
          id: "iron",
          screenshotPath: "shot-1.jpg",
          commodityName: "Iron",
          priceSell: 10,
        }),
        commodityRow({
          id: "steel-1",
          screenshotPath: "shot-1.jpg",
          commodityName: "Steel",
          priceSell: 20,
        }),
      ],
      [
        commodityRow({
          id: "steel-2",
          screenshotPath: "shot-2.jpg",
          commodityName: "Steel",
          priceSell: 21,
        }),
        commodityRow({
          id: "gas",
          screenshotPath: "shot-2.jpg",
          commodityName: "Gas",
          priceSell: 30,
        }),
      ],
    );

    expect(rows).toEqual([
      expect.objectContaining({ commodityName: "Iron", screenshotPath: "shot-1.jpg" }),
      expect.objectContaining({
        commodityName: "Steel",
        screenshotPath: "shot-1.jpg",
        priceSell: 21,
      }),
      expect.objectContaining({ commodityName: "Gas", screenshotPath: "shot-2.jpg" }),
    ]);
  });

  it("updates the existing row when the same commodity is reprocessed from the same screenshot", () => {
    const merged = mergeCommodityRows(
      [
        commodityRow({
          id: "row-a",
          screenshotPath: "shot-a.jpg",
          commodityName: "Agricium",
          priceSell: 27.5,
          scuSell: 100,
          cargoSizes: [1, 2],
        }),
      ],
      [
        commodityRow({
          id: "row-b",
          screenshotPath: "shot-a.jpg",
          commodityName: "Agricium",
          priceSell: 28,
          scuSell: 200,
          cargoSizes: [4, 8],
        }),
      ],
    );

    expect(merged).toHaveLength(1);
    expect(merged[0]).toEqual(
      expect.objectContaining({
        id: "row-a",
        screenshotPath: "shot-a.jpg",
        priceSell: 28,
        scuSell: 200,
        cargoSizes: [1, 2, 4, 8],
      }),
    );
  });

  it("resolves UEX commodity IDs from exact cached commodity names without changing screenshot ownership", () => {
    const rows = resolveCommodityIdsFromCache(
      [
        commodityRow({
          id: "a",
          screenshotPath: "shot-a.jpg",
          commodityName: "Agricium",
          idCommodity: null,
        }),
        commodityRow({
          id: "b",
          screenshotPath: "shot-b.jpg",
          commodityName: "agricium",
          idCommodity: null,
        }),
      ],
      [commodity({ id: 1, name: "Agricium" })],
    );

    expect(rows).toEqual([
      expect.objectContaining({ id: "a", screenshotPath: "shot-a.jpg", idCommodity: 1 }),
      expect.objectContaining({ id: "b", screenshotPath: "shot-b.jpg", idCommodity: 1 }),
    ]);
  });

  it("canonicalizes OCR commodity names when a compact cached name match is unique", () => {
    const rows = resolveCommodityIdsFromCache(
      [commodityRow({ id: "party", commodityName: "PARTYFAVORS", idCommodity: null })],
      [commodity({ id: 42, name: "Party Favors" })],
    );

    expect(rows).toEqual([
      expect.objectContaining({
        commodityName: "Party Favors",
        idCommodity: 42,
      }),
    ]);
  });

  it("does not canonicalize compact OCR names when the cached match is ambiguous", () => {
    const rows = resolveCommodityIdsFromCache(
      [commodityRow({ id: "ambiguous", commodityName: "A-B", idCommodity: null })],
      [commodity({ id: 1, name: "A B" }), commodity({ id: 2, name: "AB" })],
    );

    expect(rows).toEqual([
      expect.objectContaining({
        commodityName: "A-B",
        idCommodity: null,
      }),
    ]);
  });
});

function commodityRow(overrides: Partial<CommodityRow> = {}): CommodityRow {
  return {
    id: "row-1",
    screenshotPath: "shot-a.jpg",
    commodityName: "Agricium",
    idCommodity: 1,
    priceBuy: null,
    scuBuy: null,
    statusBuy: null,
    priceSell: null,
    scuSell: null,
    statusSell: null,
    cargoSizes: [],
    sourceLines: [],
    confidence: 1,
    issues: [],
    ...overrides,
  };
}

function commodity(overrides: Partial<UexCommodity>): UexCommodity {
  return {
    id: 1,
    id_parent: null,
    name: "Agricium",
    code: null,
    ...overrides,
  };
}
