import { describe, expect, it } from "vitest";
import {
  buildCommoditySubmission,
  buildScreenshotCommoditySubmissions,
  formatTerminalLabel,
} from "./uex";
import type { CommodityRow, ProcessedScreenshot, UexTerminal } from "./types";

describe("buildCommoditySubmission", () => {
  it("builds the UEX commodity data_submit payload without leaking the secret into the body", () => {
    const payload = buildCommoditySubmission({
      terminalId: 89,
      isProduction: false,
      gameVersion: " 4.7.2 ",
      screenshotBase64: " abc123 ",
      rows: [
        commodityRow({
          commodityName: "Agricium",
          idCommodity: 1,
          priceSell: 27.5,
          scuSell: 2750,
          statusSell: 5,
        }),
      ],
    });

    expect(payload).toEqual({
      id_terminal: 89,
      type: "commodity",
      is_production: 0,
      prices: [
        {
          id_commodity: 1,
          price_sell: 27.5,
          scu_sell: 2750,
          status_sell: 5,
        },
      ],
      game_version: "4.7.2",
      screenshot: "abc123",
    });
    expect(JSON.stringify(payload)).not.toContain("secret");
  });

  it("rejects rows without UEX commodity IDs before submission", () => {
    expect(() =>
      buildCommoditySubmission({
        terminalId: 89,
        isProduction: true,
        rows: [
          commodityRow({
            commodityName: "Agricium",
            idCommodity: null,
            priceSell: 27.5,
            scuSell: 2750,
            statusSell: 5,
          }),
        ],
      }),
    ).toThrow("Agricium is missing a UEX commodity ID");
  });

  it("rejects rows that mix buy and sell fields because one screenshot has one visible market side", () => {
    expect(() =>
      buildCommoditySubmission({
        terminalId: 89,
        isProduction: false,
        rows: [
          commodityRow({
            commodityName: "Tungsten",
            idCommodity: 7,
            priceBuy: 10,
            scuBuy: 0,
            statusBuy: 1,
            priceSell: 11,
            scuSell: 100,
            statusSell: 5,
          }),
        ],
      }),
    ).toThrow("Tungsten mixes buy and sell data");
  });

  it("rejects a single screenshot submission that contains both buy-side and sell-side rows", () => {
    expect(() =>
      buildCommoditySubmission({
        terminalId: 89,
        isProduction: false,
        rows: [
          commodityRow({
            id: "buy-row",
            screenshotPath: "shot-a.jpg",
            idCommodity: 7,
            priceBuy: 10,
            scuBuy: 0,
            statusBuy: 1,
          }),
          commodityRow({
            id: "sell-row",
            screenshotPath: "shot-a.jpg",
            idCommodity: 8,
            priceSell: 11,
            scuSell: 100,
            statusSell: 5,
          }),
        ],
      }),
    ).toThrow("A screenshot submission cannot mix buy and sell rows");
  });

  it("rejects invalid terminal IDs before calling UEX", () => {
    expect(() =>
      buildCommoditySubmission({
        terminalId: 0,
        isProduction: true,
        rows: [commodityRow({ priceSell: 1, scuSell: 1, statusSell: 1 })],
      }),
    ).toThrow("A UEX terminal must be selected before submission");
  });

  it("rejects UEX commodity status values outside the documented 1 through 7 range", () => {
    expect(() =>
      buildCommoditySubmission({
        terminalId: 89,
        isProduction: true,
        rows: [commodityRow({ commodityName: "Quartz", statusSell: 8 })],
      }),
    ).toThrow("Quartz has an invalid sell status");

    expect(() =>
      buildCommoditySubmission({
        terminalId: 89,
        isProduction: true,
        rows: [commodityRow({ commodityName: "Scrap", statusBuy: 0 })],
      }),
    ).toThrow("Scrap has an invalid buy status");
  });

  it("rejects payloads above UEX data_submit maximum of 500 price rows", () => {
    const rows = Array.from({ length: 501 }, (_, index) =>
      commodityRow({
        id: `row-${index}`,
        commodityName: `Commodity ${index}`,
        idCommodity: index + 1,
        priceSell: index + 1,
        scuSell: index,
        statusSell: 1,
      }),
    );

    expect(() =>
      buildCommoditySubmission({
        terminalId: 89,
        isProduction: true,
        rows,
      }),
    ).toThrow("UEX data_submit accepts at most 500 price rows per submission");
  });
});

describe("buildScreenshotCommoditySubmissions", () => {
  it("builds one UEX payload per screenshot with only that screenshot's rows and image", () => {
    const submissions = buildScreenshotCommoditySubmissions({
      terminalId: 89,
      isProduction: false,
      gameVersion: "4.8.0",
      screenshots: [
        processedScreenshot("shot-a.jpg", "screen-a"),
        processedScreenshot("shot-b.jpg", "screen-b"),
      ],
      rows: [
        commodityRow({
          id: "a",
          screenshotPath: "shot-a.jpg",
          idCommodity: 1,
          priceSell: 27.5,
          scuSell: 100,
          statusSell: 5,
        }),
        commodityRow({
          id: "b",
          screenshotPath: "shot-b.jpg",
          idCommodity: 1,
          priceSell: 28,
          scuSell: 200,
          statusSell: 6,
        }),
      ],
    });

    expect(submissions).toEqual([
      {
        screenshotPath: "shot-a.jpg",
        payload: expect.objectContaining({
          screenshot: "screen-a",
          prices: [
            expect.objectContaining({
              id_commodity: 1,
              price_sell: 27.5,
              scu_sell: 100,
              status_sell: 5,
            }),
          ],
        }),
      },
      {
        screenshotPath: "shot-b.jpg",
        payload: expect.objectContaining({
          screenshot: "screen-b",
          prices: [
            expect.objectContaining({
              id_commodity: 1,
              price_sell: 28,
              scu_sell: 200,
              status_sell: 6,
            }),
          ],
        }),
      },
    ]);
  });

  it("does not submit screenshots that have no accepted rows", () => {
    const submissions = buildScreenshotCommoditySubmissions({
      terminalId: 89,
      isProduction: false,
      screenshots: [
        processedScreenshot("shot-a.jpg", "screen-a"),
        processedScreenshot("shot-b.jpg", "screen-b"),
      ],
      rows: [
        commodityRow({
          id: "a",
          screenshotPath: "shot-a.jpg",
          idCommodity: 1,
          priceSell: 27.5,
          scuSell: 100,
          statusSell: 5,
        }),
      ],
    });

    expect(submissions.map((submission) => submission.screenshotPath)).toEqual(["shot-a.jpg"]);
  });

  it("submits buy-side and sell-side screenshots as separate screenshot-first payloads", () => {
    const submissions = buildScreenshotCommoditySubmissions({
      terminalId: 89,
      isProduction: false,
      screenshots: [
        processedScreenshot("buy-shot.jpg", "buy-image"),
        processedScreenshot("sell-shot.jpg", "sell-image"),
      ],
      rows: [
        commodityRow({
          id: "buy-row",
          screenshotPath: "buy-shot.jpg",
          idCommodity: 24,
          priceBuy: 136,
          scuBuy: 529,
          statusBuy: 1,
        }),
        commodityRow({
          id: "sell-row",
          screenshotPath: "sell-shot.jpg",
          idCommodity: 4,
          priceSell: 900,
          scuSell: 652,
          statusSell: 5,
        }),
      ],
    });

    expect(submissions).toHaveLength(2);
    expect(submissions[0].payload).toEqual(
      expect.objectContaining({
        screenshot: "buy-image",
        prices: [expect.objectContaining({ price_buy: 136, scu_buy: 529, status_buy: 1 })],
      }),
    );
    expect(submissions[1].payload).toEqual(
      expect.objectContaining({
        screenshot: "sell-image",
        prices: [expect.objectContaining({ price_sell: 900, scu_sell: 652, status_sell: 5 })],
      }),
    );
  });
});

describe("formatTerminalLabel", () => {
  it("uses the actual terminal name instead of appending the UEX database ID as a visible name", () => {
    const label = formatTerminalLabel({
      id: 18,
      name: "Central Business District",
      fullname: "Central Business District",
      displayname: "Lorville",
      city_name: "Lorville",
      planet_name: "Hurston",
      moon_name: null,
      space_station_name: null,
      outpost_name: null,
    } satisfies UexTerminal);

    expect(label).toBe("Central Business District · Lorville");
    expect(label).not.toContain("#18");
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

function processedScreenshot(path: string, screenshotBase64: string): ProcessedScreenshot {
  return {
    file: {
      path,
      filename: path,
      modifiedAtMs: 1_700_000_000_000,
    },
    ocrText: "{}",
    screenshotBase64,
  };
}
