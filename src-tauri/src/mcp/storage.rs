use super::types::MCPServerConfig;
use crate::database::{Database, DbError};
use rusqlite::params;
use std::collections::HashMap;

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
                server_url: row.get(2)?,
                auth_type: row.get(3)?,
                bearer_token: row.get(4)?,
                oauth_client_id: row.get(5)?,
                oauth_client_secret: row.get(6)?,
                custom_headers: parse_headers(&custom_headers_raw),
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
                server_url: row.get(2)?,
                auth_type: row.get(3)?,
                bearer_token: row.get(4)?,
                oauth_client_id: row.get(5)?,
                oauth_client_secret: row.get(6)?,
                custom_headers: parse_headers(&custom_headers_raw),
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
}

fn parse_headers(raw: &str) -> HashMap<String, String> {
    serde_json::from_str(raw).unwrap_or_default()
}
