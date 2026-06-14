import type { ScreenshotFile } from "./types";

export function getCaptureTimeMs(file: ScreenshotFile): number {
  return parseTimestampFromFilename(file.filename) ?? file.modifiedAtMs;
}

export function parseTimestampFromFilename(filename: string): number | null {
  const match = filename.match(
    /(?<year>20\d{2})[-_]?((?<month>\d{2}))[-_]?((?<day>\d{2}))(?:[-_\s]?((?<hour>\d{2}))[-_]?((?<minute>\d{2}))[-_]?((?<second>\d{2})))?/,
  );
  if (!match?.groups) {
    return null;
  }

  const year = Number(match.groups.year);
  const month = Number(match.groups.month);
  const day = Number(match.groups.day);
  const hour = Number(match.groups.hour ?? 0);
  const minute = Number(match.groups.minute ?? 0);
  const second = Number(match.groups.second ?? 0);
  const timestamp = Date.UTC(year, month - 1, day, hour, minute, second);

  return Number.isFinite(timestamp) ? timestamp : null;
}
