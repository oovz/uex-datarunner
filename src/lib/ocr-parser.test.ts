import { describe, expect, it } from "vitest";
import { parseCommodityOcrText, stripModelReasoning } from "./ocr-parser";

describe("parseCommodityOcrText", () => {
  it("ignores model reasoning prose when no structured JSON object is returned", () => {
    const rows = parseCommodityOcrText(
      [
        "The user wants me to identify the market side and extract commodity details.",
        "1. Identify Market Side: I can see this is a SELL terminal.",
        "2. The commodities are probably Agricium and Processed Food.",
      ].join("\n"),
      "shot-a.jpg",
    );

    expect(rows).toEqual([]);
  });

  it("parses structured AI JSON without guessing missing fields", () => {
    const rows = parseCommodityOcrText(
      JSON.stringify({
        marketSide: "sell",
        commodities: [
          {
            name: "Beryl",
            status: "Out of Stock",
            scu: "20",
            pricePerScu: "18,000",
            cargoSizes: [1, 2, 4, 64],
          },
          {
            name: "Iron",
            status: "",
            scu: "",
            pricePerScu: "",
            cargoSizes: [],
          },
        ],
      }),
      "shot-a.jpg",
    );

    expect(rows).toEqual([
      expect.objectContaining({
        commodityName: "Beryl",
        priceBuy: null,
        scuBuy: null,
        statusBuy: null,
        priceSell: 18000,
        scuSell: 20,
        statusSell: 1,
        cargoSizes: [1, 2, 4],
        issues: [],
      }),
      expect.objectContaining({
        commodityName: "Iron",
        priceSell: null,
        scuSell: null,
        statusSell: null,
        cargoSizes: [],
        issues: ["Missing price", "Missing SCU", "Missing inventory status"],
      }),
    ]);
  });

  it("strips a <think> reasoning block and reads the JSON that follows", () => {
    const rows = parseCommodityOcrText(
      [
        "<think>",
        "The user wants commodities. This is a Local Market Value (sell) panel.",
        "I should read Quartz.",
        "</think>",
        JSON.stringify({
          marketSide: "sell",
          commodities: [
            {
              name: "Quartz",
              status: "out of stock",
              scu: 0,
              pricePerScu: 4000,
              cargoSizes: [1, 2, 4],
            },
          ],
        }),
      ].join("\n"),
      "shot-think.jpg",
    );

    expect(rows).toEqual([
      expect.objectContaining({
        commodityName: "Quartz",
        priceSell: 4000,
        scuSell: 0,
        statusSell: 1,
        cargoSizes: [1, 2, 4],
        issues: [],
      }),
    ]);
  });

  it("extracts the JSON object even when the model wraps it in stray prose", () => {
    const rows = parseCommodityOcrText(
      'Here is the result: {"marketSide":"sell","commodities":[{"name":"Iron","status":"low","scu":12,"pricePerScu":3500,"cargoSizes":[]}]} Done.',
    );

    expect(rows).toEqual([
      expect.objectContaining({
        commodityName: "Iron",
        priceSell: 3500,
        scuSell: 12,
        statusSell: 3,
      }),
    ]);
  });

  it("assigns buy-side fields when the document marketSide is buy", () => {
    const rows = parseCommodityOcrText(
      JSON.stringify({
        marketSide: "buy",
        commodities: [
          {
            name: "Scrap",
            status: "full",
            scu: 2100,
            pricePerScu: 2990,
            cargoSizes: [1, 2, 4, 8, 16],
          },
        ],
      }),
    );

    expect(rows).toEqual([
      expect.objectContaining({
        commodityName: "Scrap",
        priceBuy: 2990,
        scuBuy: 2100,
        statusBuy: 7,
        priceSell: null,
        scuSell: null,
        statusSell: null,
        cargoSizes: [1, 2, 4, 8, 16],
      }),
    ]);
  });

  it("maps English and Chinese status labels to the UEX 1-7 scale", () => {
    const cases = [
      ["out of stock", 1],
      ["库存已空", 1],
      ["very low", 2],
      ["库存中等", 4],
      ["high", 5],
      ["库存已满", 7],
      ["maximum", 7],
    ] as const;

    for (const [label, expected] of cases) {
      const [row] = parseCommodityOcrText(
        JSON.stringify({
          marketSide: "sell",
          commodities: [
            { name: "Tungsten", status: label, scu: 0, pricePerScu: 10000, cargoSizes: [] },
          ],
        }),
      );
      expect(row).toEqual(expect.objectContaining({ statusSell: expected }));
    }
  });

  it("drops cargo sizes outside the allowed 1,2,4,8,16,24,32 set", () => {
    const [row] = parseCommodityOcrText(
      JSON.stringify({
        marketSide: "sell",
        commodities: [
          {
            name: "Beryl",
            status: "out of stock",
            scu: 20,
            pricePerScu: 18000,
            cargoSizes: [1, 2, 3, 4, 64],
          },
        ],
      }),
    );

    expect(row.cargoSizes).toEqual([1, 2, 4]);
  });

  it("returns no rows when the output has no structured JSON object", () => {
    expect(parseCommodityOcrText("<think>budget cut before any JSON was produced")).toEqual([]);
    expect(parseCommodityOcrText("just some prose with no object")).toEqual([]);
  });

  it("returns an empty list for an empty commodities array", () => {
    expect(parseCommodityOcrText(JSON.stringify({ marketSide: "sell", commodities: [] }))).toEqual(
      [],
    );
  });
});

describe("stripModelReasoning", () => {
  it("removes paired, unclosed, and orphan think tags", () => {
    expect(stripModelReasoning("<think>reason</think>answer")).toBe("answer");
    expect(stripModelReasoning("prefix<think>reason</think>")).toBe("prefix");
    expect(stripModelReasoning("keep<think>still thinking")).toBe("keep");
    expect(stripModelReasoning("dangling</think>final")).toBe("final");
    expect(stripModelReasoning("  plain  ")).toBe("plain");
  });
});
