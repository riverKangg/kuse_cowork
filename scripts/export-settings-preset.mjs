import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const repoRoot = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const outputPath = path.join(repoRoot, "src-tauri", "default-settings.json");

function defaultDbPath() {
  const home = os.homedir();
  switch (process.platform) {
    case "darwin":
      return path.join(home, "Library", "Application Support", "kuse-cowork", "kuse-cowork.db");
    case "win32": {
      const appData = process.env.APPDATA ?? path.join(home, "AppData", "Roaming");
      return path.join(appData, "kuse-cowork", "kuse-cowork.db");
    }
    default:
      return path.join(home, ".local", "share", "kuse-cowork", "kuse-cowork.db");
  }
}

function runSqliteJson(dbPath, sql) {
  const raw = execFileSync("sqlite3", ["-json", dbPath, sql], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  return raw.trim() ? JSON.parse(raw) : [];
}

const dbPath = process.env.KUSE_COWORK_DB_PATH || defaultDbPath();

if (!fs.existsSync(dbPath)) {
  throw new Error(`Database not found: ${dbPath}`);
}

const rows = runSqliteJson(
  dbPath,
  `
    SELECT key, value
    FROM settings
    WHERE key IN (
      'model',
      'base_url',
      'max_tokens',
      'temperature',
      'provider',
      'openai_organization',
      'openai_project'
    )
  `
);

const map = Object.fromEntries(rows.map((row) => [row.key, row.value]));
const preset = {
  model: map.model || "custom-model",
  base_url: map.base_url || "http://localhost:8000",
  max_tokens: Number.parseInt(map.max_tokens || "4096", 10) || 4096,
  temperature: Number.parseFloat(map.temperature || "0.7") || 0.7,
  provider: map.provider || "custom",
  ...(map.openai_organization ? { openai_organization: map.openai_organization } : {}),
  ...(map.openai_project ? { openai_project: map.openai_project } : {}),
};

fs.writeFileSync(outputPath, `${JSON.stringify(preset, null, 2)}\n`, "utf8");
console.log(`Exported settings preset to ${outputPath}`);
