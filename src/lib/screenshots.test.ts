import { describe, expect, it } from "vitest";
import { parseTimestampFromFilename } from "./screenshots";

describe("parseTimestampFromFilename", () => {
  it("parses Star Citizen screenshot filenames into timestamps", () => {
    expect(parseTimestampFromFilename("ScreenShot_20260515_091000.png")).toBe(
      Date.UTC(2026, 4, 15, 9, 10, 0),
    );
    expect(parseTimestampFromFilename("ScreenShot-2026-05-15-09-10-00.png")).toBe(
      Date.UTC(2026, 4, 15, 9, 10, 0),
    );
  });

  it("returns null for filenames without timestamps", () => {
    expect(parseTimestampFromFilename("image.png")).toBeNull();
    expect(parseTimestampFromFilename("screenshot.jpg")).toBeNull();
  });
});
