import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const workspaceRoot = path.resolve(__dirname, "../..");

function readProjectFile(relativePath: string): string {
  return readFileSync(path.join(workspaceRoot, relativePath), "utf8");
}

function extractFrontendCommands(): string[] {
  const appSource = readProjectFile("src/App.tsx");
  const backendSource = readProjectFile("src/lib/backend.ts");
  const commandPattern = /(?:callBackend|case)\s*(?:<[^>]+>)?\s*\(?\s*["']([a-z_]+)["']/g;
  const commands = [
    ...appSource.matchAll(commandPattern),
    ...backendSource.matchAll(commandPattern),
  ].map((match) => match[1]);

  return Array.from(new Set(commands)).sort();
}

function extractBuildManifestCommands(): string[] {
  const buildSource = readProjectFile("src-tauri/build.rs");
  const commandsCall = buildSource.match(
    /AppManifest::new\(\)\s*\.commands\(\s*&\[(?<commands>[\s\S]*?)\]\s*\)/,
  );
  if (!commandsCall?.groups?.commands) {
    return [];
  }

  return Array.from(commandsCall.groups.commands.matchAll(/"([a-z_]+)"/g))
    .map((match) => match[1])
    .sort();
}

describe("Tauri v2 command permissions", () => {
  it("declares every frontend-invoked backend command in the build manifest and default capability", () => {
    const frontendCommands = extractFrontendCommands();
    const manifestCommands = extractBuildManifestCommands();
    const capability = JSON.parse(readProjectFile("src-tauri/capabilities/default.json")) as {
      permissions: string[];
    };
    const capabilityPermissions = new Set(capability.permissions);

    expect(manifestCommands).toEqual(frontendCommands);
    for (const command of frontendCommands) {
      expect(capabilityPermissions.has(`allow-${command.replaceAll("_", "-")}`), command).toBe(
        true,
      );
    }
  });
});
