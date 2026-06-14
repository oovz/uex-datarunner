import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const workspaceRoot = path.resolve(__dirname, "../..");

function readProjectFile(relativePath: string): string {
  return readFileSync(path.join(workspaceRoot, relativePath), "utf8");
}

describe("desktop shell configuration", () => {
  it("uses a compact, draggable custom Tauri window shell", () => {
    const tauriConfig = JSON.parse(readProjectFile("src-tauri/tauri.conf.json")) as {
      app: {
        windows: Array<{ width: number; height: number; minWidth: number; minHeight: number }>;
      };
    };
    const [mainWindow] = tauriConfig.app.windows;
    const styles = readProjectFile("src/styles.css");

    expect(mainWindow.width).toBeGreaterThan(mainWindow.height);
    expect(mainWindow.width).toBeGreaterThanOrEqual(1180);
    expect(mainWindow.minWidth).toBeGreaterThan(mainWindow.minHeight);
    expect(styles).toContain("app-region: drag");
    expect(styles).toContain("app-region: no-drag");
    expect(styles).toContain("min-width: 980px");
    expect(styles).toContain("min-height: 600px");
  });
});
