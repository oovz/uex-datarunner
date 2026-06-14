import type { CommodityRow, ProcessedScreenshot, UexDatarunnerDataType } from "./types";

const MAX_UEX_DATA_SUBMIT_PRICE_ROWS = 500;
export type MarketSide = "buy" | "sell";

export type BuildCommoditySubmissionInput = {
  terminalId: number;
  isProduction: boolean;
  gameVersion?: string | null;
  screenshotBase64?: string | null;
  rows: CommodityRow[];
};

export type CommoditySubmissionPayload = {
  id_terminal: number;
  type: UexDatarunnerDataType;
  is_production: 0 | 1;
  prices: Array<{
    id_commodity: number;
    price_buy?: number;
    price_sell?: number;
    scu_buy?: number;
    scu_sell?: number;
    status_buy?: number;
    status_sell?: number;
    quality?: number;
  }>;
  container_sizes?: string;
  game_version?: string;
  screenshot?: string;
};

export type BuildScreenshotCommoditySubmissionsInput = Omit<
  BuildCommoditySubmissionInput,
  "rows" | "screenshotBase64"
> & {
  rows: CommodityRow[];
  screenshots: ProcessedScreenshot[];
};

export type ScreenshotCommoditySubmission = {
  screenshotPath: string | null;
  payload: CommoditySubmissionPayload;
};

export function buildScreenshotCommoditySubmissions(
  input: BuildScreenshotCommoditySubmissionsInput,
): ScreenshotCommoditySubmission[] {
  const screenshotByPath = new Map(
    input.screenshots.map((screenshot) => [screenshot.file.path, screenshot]),
  );
  const rowsByScreenshot = groupRowsByScreenshot(input.rows);

  return Array.from(rowsByScreenshot, ([screenshotPath, rows]) => {
    const screenshot = screenshotPath ? screenshotByPath.get(screenshotPath) : null;
    return {
      screenshotPath,
      payload: buildCommoditySubmission({
        terminalId: input.terminalId,
        isProduction: input.isProduction,
        gameVersion: input.gameVersion,
        screenshotBase64: screenshot?.screenshotBase64 ?? null,
        rows,
      }),
    };
  });
}

export function buildCommoditySubmission(
  input: BuildCommoditySubmissionInput,
): CommoditySubmissionPayload {
  if (!Number.isInteger(input.terminalId) || input.terminalId <= 0) {
    throw new Error("A UEX terminal must be selected before submission");
  }

  const submissionSide = getSubmissionSide(input.rows);
  const prices = input.rows.map((row) => {
    if (row.idCommodity === null) {
      throw new Error(`${row.commodityName} is missing a UEX commodity ID`);
    }

    const side = getRowMarketSide(row);
    if (!side) {
      throw new Error(`${row.commodityName} has no buy or sell data to submit`);
    }
    if (side !== submissionSide) {
      throw new Error("A screenshot submission cannot mix buy and sell rows");
    }

    const price: CommoditySubmissionPayload["prices"][number] = {
      id_commodity: row.idCommodity,
    };

    if (side === "buy") {
      validateCommodityStatus(row.commodityName, "buy", row.statusBuy);
      if (row.priceBuy !== null) {
        price.price_buy = row.priceBuy;
      }
      if (row.scuBuy !== null) {
        price.scu_buy = row.scuBuy;
      }
      if (row.statusBuy !== null) {
        price.status_buy = row.statusBuy;
      }
    } else {
      validateCommodityStatus(row.commodityName, "sell", row.statusSell);
      if (row.priceSell !== null) {
        price.price_sell = row.priceSell;
      }
      if (row.scuSell !== null) {
        price.scu_sell = row.scuSell;
      }
      if (row.statusSell !== null) {
        price.status_sell = row.statusSell;
      }
    }

    return price;
  });

  if (prices.length === 0) {
    throw new Error("There are no commodity rows to submit");
  }
  if (prices.length > MAX_UEX_DATA_SUBMIT_PRICE_ROWS) {
    throw new Error(
      `UEX data_submit accepts at most ${MAX_UEX_DATA_SUBMIT_PRICE_ROWS} price rows per submission`,
    );
  }

  const payload: CommoditySubmissionPayload = {
    id_terminal: input.terminalId,
    type: "commodity",
    is_production: input.isProduction ? 1 : 0,
    prices,
  };

  if (input.gameVersion?.trim()) {
    payload.game_version = input.gameVersion.trim();
  }

  const cargoSizes = uniqueCargoSizes(input.rows);
  if (cargoSizes.length > 0) {
    payload.container_sizes = cargoSizes.join(",");
  }

  if (input.screenshotBase64?.trim()) {
    payload.screenshot = input.screenshotBase64.trim();
  }

  return payload;
}

export function getRowMarketSide(row: CommodityRow): MarketSide | null {
  const hasBuy = row.priceBuy !== null || row.scuBuy !== null || row.statusBuy !== null;
  const hasSell = row.priceSell !== null || row.scuSell !== null || row.statusSell !== null;

  if (hasBuy && hasSell) {
    throw new Error(
      `${row.commodityName} mixes buy and sell data. Review one market side per row.`,
    );
  }

  if (hasBuy) {
    return "buy";
  }
  if (hasSell) {
    return "sell";
  }
  return null;
}

export function getRowMarketValues(row: CommodityRow, side = getRowMarketSide(row) ?? "sell") {
  return side === "buy"
    ? { price: row.priceBuy, scu: row.scuBuy, status: row.statusBuy }
    : { price: row.priceSell, scu: row.scuSell, status: row.statusSell };
}

export function patchRowMarketValues(
  row: CommodityRow,
  side: MarketSide,
  values: Partial<{ price: number | null; scu: number | null; status: number | null }>,
): CommodityRow {
  const base: CommodityRow =
    side === "buy"
      ? { ...row, priceSell: null, scuSell: null, statusSell: null }
      : { ...row, priceBuy: null, scuBuy: null, statusBuy: null };

  if (side === "buy") {
    return {
      ...base,
      priceBuy: values.price === undefined ? base.priceBuy : values.price,
      scuBuy: values.scu === undefined ? base.scuBuy : values.scu,
      statusBuy: values.status === undefined ? base.statusBuy : values.status,
    };
  }

  return {
    ...base,
    priceSell: values.price === undefined ? base.priceSell : values.price,
    scuSell: values.scu === undefined ? base.scuSell : values.scu,
    statusSell: values.status === undefined ? base.statusSell : values.status,
  };
}

function getSubmissionSide(rows: CommodityRow[]): MarketSide {
  let side: MarketSide | null = null;
  for (const row of rows) {
    const rowSide = getRowMarketSide(row);
    if (!rowSide) {
      continue;
    }
    if (side && rowSide !== side) {
      throw new Error("A screenshot submission cannot mix buy and sell rows");
    }
    side = rowSide;
  }

  if (!side) {
    throw new Error("There are no commodity rows to submit");
  }

  return side;
}

function groupRowsByScreenshot(rows: CommodityRow[]): Map<string | null, CommodityRow[]> {
  const groups = new Map<string | null, CommodityRow[]>();
  for (const row of rows) {
    const key = row.screenshotPath;
    const group = groups.get(key) ?? [];
    group.push(row);
    groups.set(key, group);
  }
  return groups;
}

function uniqueCargoSizes(rows: CommodityRow[]): number[] {
  const allowed = new Set([1, 2, 4, 8, 16, 24, 32]);
  return Array.from(
    new Set(rows.flatMap((row) => row.cargoSizes).filter((size) => allowed.has(size))),
  ).sort((left, right) => left - right);
}

function validateCommodityStatus(
  commodityName: string,
  side: "buy" | "sell",
  status: number | null,
): void {
  if (status === null) {
    return;
  }
  if (!Number.isInteger(status) || status < 1 || status > 7) {
    throw new Error(
      `${commodityName} has an invalid ${side} status. UEX commodity statuses must be 1 through 7.`,
    );
  }
}

// Documented UEX /data_submit response statuses mapped to readable messages.
// Source: https://uexcorp.space/api/documentation/id/post_data_submit/
const UEX_SUBMIT_STATUS_MESSAGES: Record<string, string> = {
  ok: "Accepted",
  invalid_secret_key: "UEX rejected the secret key. Re-check it in Settings.",
  missing_secret_key: "No UEX secret key configured.",
  user_not_found: "UEX datarunner account not found.",
  user_not_allowed: "This UEX account is restricted or is not a datarunner.",
  user_disabled: "This UEX datarunner account is banned or blocked.",
  duplicated_report: "UEX already has a matching report for this terminal in the last 5 minutes.",
  too_many_reports: "Rate limited: more than 1000 reports were sent in the last 30 minutes.",
  max_rows_exceeded: "Too many rows: UEX accepts at most 500 prices per submission.",
  invalid_id_commodity: "A commodity ID was not recognised by UEX.",
  missing_id_commodity: "A row is missing its UEX commodity ID.",
  missing_id_terminal: "No terminal was selected.",
  terminal_not_found: "UEX did not recognise the selected terminal.",
  not_allowed_player_terminal: "This is a player-owned terminal you cannot update.",
  invalid_status_buy: "An invalid buy inventory status was sent.",
  invalid_status_sell: "An invalid sell inventory status was sent.",
  invalid_quality: "A commodity quality value was outside the 0-1000 range.",
  has_no_prices_and_no_is_missing_set: "A row has neither a price nor an is-missing flag.",
  has_both_price_buy_and_price_sell: "A row mixes buy and sell prices.",
  has_both_scu_buy_and_scu_sell: "A row mixes buy and sell SCU values.",
  invalid_game_version: "The game version is not accepted by UEX (LIVE or PTU only).",
  ptu_reports_not_allowed: "UEX is not accepting PTU reports right now.",
  screenshot_required: "UEX requires a screenshot from this datarunner.",
  screenshot_length_exceeds_limit: "The screenshot exceeds the 10 MB limit.",
  invalid_input: "UEX could not parse the submission (invalid JSON).",
  service_unavailable: "UEX data service is temporarily unavailable. Try again shortly.",
};

/** Maps a UEX `/data_submit` response status to a human-readable message. */
export function describeUexSubmitStatus(status: string): string {
  return UEX_SUBMIT_STATUS_MESSAGES[status] ?? `UEX returned an unexpected status: ${status}`;
}

export function formatTerminalLabel(terminal: {
  id: number;
  displayname: string | null;
  name: string;
  fullname?: string | null;
  city_name: string | null;
  planet_name: string | null;
  moon_name: string | null;
  space_station_name: string | null;
  outpost_name: string | null;
}): string {
  const primary =
    terminal.name.trim() ||
    terminal.fullname?.trim() ||
    terminal.displayname?.trim() ||
    "Unknown terminal";
  const location = [
    terminal.displayname,
    terminal.city_name,
    terminal.outpost_name,
    terminal.space_station_name,
    terminal.moon_name,
    terminal.planet_name,
  ].find((part) => {
    const value = part?.trim();
    return value && value.toLowerCase() !== primary.toLowerCase();
  });

  return `${primary}${location ? ` · ${location}` : ""}`;
}
