use super::types::MCPServerConfig;
use crate::database::{Database, DbError};
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

const DEFAULT_MCP_PRESETS_JSON: &str = include_str!("../../default-mcp-servers.json");
const MCP_PRESETS_SEEDED_KEY: &str = "mcp_presets_seeded";

impl Database {
    pub fn create_mcp_tables(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                server_url TEXT NOT NULL,
                auth_type TEXT NOT NULL DEFAULT 'none',
                bearer_token TEXT,
                oauth_client_id TEXT,
                oauth_client_secret TEXT,
                custom_headers TEXT NOT NULL DEFAULT '{}',
                enabled BOOLEAN NOT NULL DEFAULT 0,
                created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP NOT NULL
            )",
            [],
        )?;

        let migrations = [
            "ALTER TABLE mcp_servers ADD COLUMN auth_type TEXT NOT NULL DEFAULT 'none'",
            "ALTER TABLE mcp_servers ADD COLUMN bearer_token TEXT",
            "ALTER TABLE mcp_servers ADD COLUMN custom_headers TEXT NOT NULL DEFAULT '{}'",
        ];

        for sql in migrations {
            match conn.execute(sql, []) {
                Ok(_) => {}
                Err(rusqlite::Error::SqliteFailure(_, Some(message)))
                    if message.contains("duplicate column name") => {}
                Err(err) => return Err(err.into()),
            }
        }

        Ok(())
    }

    pub fn save_mcp_server(&self, config: &MCPServerConfig) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let sanitized_server_url = config.server_url.trim().to_string();

        conn.execute(
            "INSERT OR REPLACE INTO mcp_servers
             (id, name, server_url, auth_type, bearer_token, oauth_client_id, oauth_client_secret, custom_headers, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                config.id,
                config.name,
                sanitized_server_url,
                config.auth_type,
                config.bearer_token,
                config.oauth_client_id,
                config.oauth_client_secret,
                serde_json::to_string(&config.custom_headers).unwrap_or_else(|_| "{}".to_string()),
                config.enabled,
                config.created_at,
                config.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_mcp_servers(&self) -> Result<Vec<MCPServerConfig>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, name, server_url, auth_type, bearer_token, oauth_client_id, oauth_client_secret, custom_headers, enabled, created_at, updated_at
             FROM mcp_servers ORDER BY name"
        )?;

        let server_iter = stmt.query_map([], |row| {
            let custom_headers_raw: String = row.get(7)?;
            Ok(MCPServerConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                server_url: row.get::<_, String>(2)?.trim().to_string(),
                auth_type: row.get(3)?,
                bearer_token: row.get(4)?,
                oauth_client_id: row.get(5)?,
                oauth_client_secret: row.get(6)?,
                custom_headers: parse_headers(&custom_headers_raw),
                custom_headers_updated: false,
                has_bearer_token: false,
                has_oauth_client_secret: false,
                custom_header_keys: Vec::new(),
                enabled: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        let mut servers = Vec::new();
        for server in server_iter {
            servers.push(server?);
        }
        Ok(servers)
    }

    pub fn get_mcp_server(&self, id: &str) -> Result<Option<MCPServerConfig>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, name, server_url, auth_type, bearer_token, oauth_client_id, oauth_client_secret, custom_headers, enabled, created_at, updated_at
             FROM mcp_servers WHERE id = ?1"
        )?;

        let mut server_iter = stmt.query_map([id], |row| {
            let custom_headers_raw: String = row.get(7)?;
            Ok(MCPServerConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                server_url: row.get::<_, String>(2)?.trim().to_string(),
                auth_type: row.get(3)?,
                bearer_token: row.get(4)?,
                oauth_client_id: row.get(5)?,
                oauth_client_secret: row.get(6)?,
                custom_headers: parse_headers(&custom_headers_raw),
                custom_headers_updated: false,
                has_bearer_token: false,
                has_oauth_client_secret: false,
                custom_header_keys: Vec::new(),
                enabled: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        match server_iter.next() {
            Some(server) => Ok(Some(server?)),
            None => Ok(None),
        }
    }

    pub fn delete_mcp_server(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        conn.execute("DELETE FROM mcp_servers WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_mcp_server_enabled(&self, id: &str, enabled: bool) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        conn.execute(
            "UPDATE mcp_servers SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![enabled, chrono::Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn seed_default_mcp_servers_if_needed(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let already_seeded = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [MCP_PRESETS_SEEDED_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .is_some_and(|value| value == "true");

        if already_seeded {
            return Ok(());
        }

        let existing_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM mcp_servers", [], |row| row.get(0))?;

        if existing_count > 0 {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                [MCP_PRESETS_SEEDED_KEY, "true"],
            )?;
            return Ok(());
        }

        let presets: Vec<MCPServerPreset> =
            serde_json::from_str(DEFAULT_MCP_PRESETS_JSON).unwrap_or_default();

        for preset in presets {
            let config = preset.into_server_config();
            conn.execute(
                "INSERT OR REPLACE INTO mcp_servers
                 (id, name, server_url, auth_type, bearer_token, oauth_client_id, oauth_client_secret, custom_headers, enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    config.id,
                    config.name,
                    config.server_url,
                    config.auth_type,
                    config.bearer_token,
                    config.oauth_client_id,
                    config.oauth_client_secret,
                    serde_json::to_string(&config.custom_headers)
                        .unwrap_or_else(|_| "{}".to_string()),
                    config.enabled,
                    config.created_at,
                    config.updated_at,
                ],
            )?;
        }

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            [MCP_PRESETS_SEEDED_KEY, "true"],
        )?;

        Ok(())
    }
}

fn parse_headers(raw: &str) -> HashMap<String, String> {
    serde_json::from_str(raw).unwrap_or_default()
}

#[derive(serde::Deserialize, Default)]
struct MCPServerPreset {
    id: Option<String>,
    name: String,
    server_url: String,
    #[serde(default = "default_auth_type")]
    auth_type: String,
    #[serde(default)]
    oauth_client_id: Option<String>,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    custom_header_keys: Vec<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
}

impl MCPServerPreset {
    fn into_server_config(self) -> MCPServerConfig {
        let now = chrono::Utc::now().to_rfc3339();
        let created_at = self.created_at.unwrap_or_else(|| now.clone());
        let updated_at = self.updated_at.unwrap_or_else(|| now.clone());
        let has_bearer_token = self.auth_type == "bearer";
        let has_oauth_client_secret = self.auth_type == "oauth_client_credentials";

        MCPServerConfig {
            id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: self.name,
            server_url: self.server_url.trim().to_string(),
            auth_type: self.auth_type,
            bearer_token: None,
            oauth_client_id: self.oauth_client_id.filter(|value| !value.trim().is_empty()),
            oauth_client_secret: None,
            custom_headers: HashMap::new(),
            custom_headers_updated: false,
            has_bearer_token,
            has_oauth_client_secret,
            custom_header_keys: self.custom_header_keys,
            enabled: self.enabled,
            created_at,
            updated_at,
        }
    }
}

fn default_auth_type() -> String {
    "none".to_string()
}
