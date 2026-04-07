import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const repoRoot = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const outputPath = path.join(repoRoot, "src-tauri", "default-mcp-servers.json");

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
    SELECT
      id,
      name,
      server_url,
      auth_type,
      oauth_client_id,
      enabled,
      created_at,
      updated_at,
      custom_headers,
      LENGTH(TRIM(COALESCE(bearer_token, ''))) > 0 AS has_bearer_token,
      LENGTH(TRIM(COALESCE(oauth_client_secret, ''))) > 0 AS has_oauth_client_secret
    FROM mcp_servers
    ORDER BY name
  `
);

const presets = rows.map((row) => {
  const customHeaders =
    typeof row.custom_headers === "string" && row.custom_headers.trim()
      ? JSON.parse(row.custom_headers)
      : {};

  return {
    id: row.id,
    name: row.name,
    server_url: row.server_url,
    auth_type: row.auth_type || "none",
    oauth_client_id: row.oauth_client_id || undefined,
    enabled: Boolean(row.enabled),
    custom_header_keys: Object.keys(customHeaders).sort(),
    created_at: row.created_at,
    updated_at: row.updated_at,
    has_bearer_token: Boolean(row.has_bearer_token),
    has_oauth_client_secret: Boolean(row.has_oauth_client_secret),
  };
}).map(({ has_bearer_token, has_oauth_client_secret, ...preset }) => preset);

fs.writeFileSync(outputPath, `${JSON.stringify(presets, null, 2)}\n`, "utf8");

console.log(`Exported ${presets.length} MCP preset(s) to ${outputPath}`);
