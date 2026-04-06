import { Component, For, createSignal, onMount, createMemo } from "solid-js";
import {
  MCPServerConfig,
  MCPServerStatus,
  listMCPServers,
  saveMCPServer,
  deleteMCPServer,
  connectMCPServer,
  disconnectMCPServer,
  getMCPServerStatuses
} from "../lib/mcp-api";
import "./MCPSettings.css";

type MCPAuthType =
  | "none"
  | "bearer"
  | "oauth_client_credentials";

function formatAuthLabel(authType?: string): string {
  switch (authType) {
    case "bearer":
      return "Bearer Token";
    case "oauth_client_credentials":
      return "OAuth Client Credentials";
    default:
      return "None";
  }
}

interface MCPSettingsProps {
  onClose: () => void;
}

const MCPSettings: Component<MCPSettingsProps> = (props) => {
  const [servers, setServers] = createSignal<MCPServerConfig[]>([]);
  const [statuses, setStatuses] = createSignal<MCPServerStatus[]>([]);
  const [showAddForm, setShowAddForm] = createSignal(false);
  const [editingServer, setEditingServer] = createSignal<MCPServerConfig | null>(null);
  const [loading, setLoading] = createSignal(false);

  // Form state
  const [formData, setFormData] = createSignal({
    name: "",
    serverUrl: "",
    authType: "none" as MCPAuthType,
    bearerToken: "",
    oauthClientId: "",
    oauthClientSecret: "",
    customHeaders: "",
  });

  const mergedData = createMemo(() => {
    const statusMap = new Map(statuses().map(s => [s.id, s]));
    return servers().map(server => ({
      server,
      status: statusMap.get(server.id)
    }));
  });

  onMount(async () => {
    await refreshData();
  });

  const refreshData = async () => {
    try {
      setLoading(true);
      const [serverList, statusList] = await Promise.all([
        listMCPServers(),
        getMCPServerStatuses()
      ]);
      setServers(serverList);
      setStatuses(statusList);
    } catch (err) {
      console.error("Failed to load MCP data:", err);
    } finally {
      setLoading(false);
    }
  };

  const resetForm = () => {
    setFormData({
      name: "",
      serverUrl: "",
      authType: "none" as MCPAuthType,
      bearerToken: "",
      oauthClientId: "",
      oauthClientSecret: "",
      customHeaders: "",
    });
    setEditingServer(null);
    setShowAddForm(false);
  };

  const startEdit = (server: MCPServerConfig) => {
    setFormData({
      name: server.name,
      serverUrl: server.server_url || "",
      authType: (server.auth_type || "none") as MCPAuthType,
      bearerToken: "",
      oauthClientId: server.oauth_client_id || "",
      oauthClientSecret: "",
      customHeaders: "",
    });
    setEditingServer(server);
    setShowAddForm(true);
  };

  const handleSave = async () => {
    try {
      const data = formData();

      if (!data.name.trim()) {
        alert("Server name is required");
        return;
      }

      if (!data.serverUrl.trim()) {
        alert("Server URL is required");
        return;
      }

      let parsedHeaders: Record<string, string> = {};
      if (data.customHeaders.trim()) {
        try {
          const candidate = JSON.parse(data.customHeaders);
          if (candidate && typeof candidate === "object" && !Array.isArray(candidate)) {
            parsedHeaders = Object.fromEntries(
              Object.entries(candidate).map(([key, value]) => [key, String(value)])
            );
          } else {
            alert("Custom headers must be a JSON object");
            return;
          }
        } catch {
          alert("Custom headers must be valid JSON");
          return;
        }
      }

      if (data.authType === "bearer" &&
        !data.bearerToken.trim() &&
        !(editingServer()?.has_bearer_token)) {
        alert("Bearer token is required for bearer auth");
        return;
      }

      if (data.authType === "oauth_client_credentials" &&
        (!data.oauthClientId.trim() ||
          (!data.oauthClientSecret.trim() && !(editingServer()?.has_oauth_client_secret)))) {
        alert("OAuth client ID and secret are required for OAuth auth");
        return;
      }

      const config: MCPServerConfig = {
        id: editingServer()?.id || crypto.randomUUID(),
        name: data.name,
        server_url: data.serverUrl,
        auth_type: data.authType,
        bearer_token: data.authType === "bearer" ? data.bearerToken.trim() || undefined : undefined,
        oauth_client_id: data.authType === "oauth_client_credentials" ? data.oauthClientId.trim() || undefined : undefined,
        oauth_client_secret: data.authType === "oauth_client_credentials" ? data.oauthClientSecret.trim() || undefined : undefined,
        custom_headers: parsedHeaders,
        custom_headers_updated: data.customHeaders.trim() !== "",
        has_bearer_token: editingServer()?.has_bearer_token ?? false,
        has_oauth_client_secret: editingServer()?.has_oauth_client_secret ?? false,
        custom_header_keys: editingServer()?.custom_header_keys || [],
        enabled: editingServer()?.enabled ?? true,
        created_at: editingServer()?.created_at || new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      await saveMCPServer(config);
      await refreshData();
      resetForm();
    } catch (err) {
      console.error("Failed to save server:", err);
      alert("Failed to save server configuration");
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Are you sure you want to delete this MCP server?")) {
      return;
    }

    try {
      await deleteMCPServer(id);
      await refreshData();
    } catch (err) {
      console.error("Failed to delete server:", err);
      alert("Failed to delete server");
    }
  };

  const handleToggleConnection = async (server: MCPServerConfig, currentStatus?: MCPServerStatus) => {
    try {
      if (currentStatus?.status === "Connected") {
        await disconnectMCPServer(server.id);
      } else {
        await connectMCPServer(server.id);
      }
      await refreshData();
    } catch (err) {
      console.error("Failed to toggle connection:", err);
      alert("Failed to connect/disconnect server");
    }
  };

  const getStatusColor = (status?: string) => {
    switch (status) {
      case "Connected": return "green";
      case "Connecting": return "orange";
      case "Error": return "red";
      default: return "gray";
    }
  };

  return (
    <div class="mcp-settings">
      <div class="mcp-settings-header">
        <h2>MCP Settings</h2>
        <div class="header-actions">
          <button class="add-btn" onClick={() => setShowAddForm(true)}>
            Add Server
          </button>
          <button class="refresh-btn" onClick={refreshData} disabled={loading()}>
            {loading() ? "Loading..." : "Refresh"}
          </button>
          <button class="close-btn" onClick={props.onClose}>
            Close
          </button>
        </div>
      </div>

      <div class="mcp-settings-content">
        {showAddForm() && (
          <div class="add-form">
            <h3>{editingServer() ? "Edit Server" : "Add MCP Server"}</h3>

            <div class="form-group">
              <label>Name</label>
              <input
                type="text"
                value={formData().name}
                onInput={(e) => setFormData(prev => ({ ...prev, name: e.currentTarget.value }))}
                placeholder="Server name"
              />
            </div>

            <div class="form-group">
              <label>Remote MCP server URL</label>
              <input
                type="url"
                value={formData().serverUrl}
                onInput={(e) => setFormData(prev => ({ ...prev, serverUrl: e.currentTarget.value }))}
                placeholder="https://your-mcp-server.com"
              />
            </div>

            <div class="form-group">
              <label>Authentication</label>
              <select
                value={formData().authType}
                onChange={(e) => setFormData(prev => ({ ...prev, authType: e.currentTarget.value as MCPAuthType }))}
              >
                <option value="none">None</option>
                <option value="bearer">Bearer Token</option>
                <option value="oauth_client_credentials">OAuth Client Credentials</option>
              </select>
            </div>

            {formData().authType === "bearer" && (
              <div class="form-group">
                <label>Bearer Token</label>
                <input
                  type="password"
                  value={formData().bearerToken}
                  onInput={(e) => setFormData(prev => ({ ...prev, bearerToken: e.currentTarget.value }))}
                  placeholder={editingServer()?.has_bearer_token ? "Leave blank to keep existing token" : "your-bearer-token"}
                />
              </div>
            )}

            <details class="advanced-settings">
              <summary>Advanced settings</summary>
              <div class="advanced-content">
                {formData().authType === "oauth_client_credentials" && (
                  <>
                    <div class="form-group">
                      <label>OAuth Client ID</label>
                      <input
                        type="text"
                        value={formData().oauthClientId}
                        onInput={(e) => setFormData(prev => ({ ...prev, oauthClientId: e.currentTarget.value }))}
                        placeholder="your-oauth-client-id"
                      />
                    </div>

                    <div class="form-group">
                      <label>OAuth Client Secret</label>
                      <input
                        type="password"
                        value={formData().oauthClientSecret}
                        onInput={(e) => setFormData(prev => ({ ...prev, oauthClientSecret: e.currentTarget.value }))}
                        placeholder={editingServer()?.has_oauth_client_secret ? "Leave blank to keep existing client secret" : "your-oauth-client-secret"}
                      />
                    </div>
                  </>
                )}

                <div class="form-group">
                  <label>Custom Headers (JSON)</label>
                  <textarea
                    rows={6}
                    value={formData().customHeaders}
                    onInput={(e) => setFormData(prev => ({ ...prev, customHeaders: e.currentTarget.value }))}
                    placeholder={editingServer()?.custom_header_keys.length
                      ? 'Leave blank to keep existing headers, or enter {} to clear'
                      : '{\n  "X-Example": "value"\n}'}
                  />
                  <small class="hint">
                    Use this for pass-through auth headers or server-specific requirements.
                    {editingServer()?.custom_header_keys.length
                      ? ` Existing header keys: ${editingServer()?.custom_header_keys.join(", ")}.`
                      : ""}
                  </small>
                </div>
              </div>
            </details>

            <div class="warning-text">
              <strong>Security Notice:</strong> Only use connectors from developers you trust.
              MCP servers have access to tools and data as configured, and this app cannot verify
              that they will work as intended or that they won't change.
            </div>

            <div class="form-actions">
              <button class="save-btn" onClick={handleSave}>
                {editingServer() ? "Update" : "Add"}
              </button>
              <button class="cancel-btn" onClick={resetForm}>Cancel</button>
            </div>
          </div>
        )}

        <div class="servers-list">
          <h3>MCP Servers</h3>

          {mergedData().length === 0 ? (
            <div class="empty-state">
              <p>No MCP servers configured.</p>
              <p>Add your first server to get started with MCP tools.</p>
            </div>
          ) : (
            <div class="servers-grid">
              <For each={mergedData()}>
                {({ server, status }) => (
                  <div class="server-card">
                    <div class="server-header">
                      <div class="server-info">
                        <h4>{server.name}</h4>
                        <p>{server.server_url}</p>
                      </div>
                      <div class="server-status">
                        <span
                          class={`status-badge ${getStatusColor(status?.status)}`}
                          title={status?.last_error}
                        >
                          {status?.status || "Disconnected"}
                        </span>
                      </div>
                    </div>

                    <div class="server-details">
                      <div class="detail-row">
                        <strong>Auth:</strong> {formatAuthLabel(server.auth_type)}
                      </div>

                      <div class="detail-row">
                        <strong>URL:</strong> {server.server_url}
                      </div>

                      {server.has_bearer_token && (
                        <div class="detail-row">
                          <strong>Bearer:</strong> Configured
                        </div>
                      )}

                      {server.oauth_client_id && (
                        <div class="detail-row">
                          <strong>OAuth:</strong> Configured
                        </div>
                      )}

                      {server.custom_header_keys.length > 0 && (
                        <div class="detail-row">
                          <strong>Headers:</strong> {server.custom_header_keys.join(", ")}
                        </div>
                      )}

                      {status?.tools && status.tools.length > 0 && (
                        <div class="detail-row">
                          <strong>Tools:</strong> {status.tools.map(t => t.name).join(", ")}
                        </div>
                      )}
                    </div>

                    <div class="server-actions">
                      <button
                        class={`toggle-btn ${status?.status === "Connected" ? "disconnect" : "connect"}`}
                        onClick={() => handleToggleConnection(server, status)}
                        disabled={status?.status === "Connecting"}
                      >
                        {status?.status === "Connected" ? "Disconnect" :
                         status?.status === "Connecting" ? "Connecting..." : "Connect"}
                      </button>
                      <button class="edit-btn" onClick={() => startEdit(server)}>
                        Edit
                      </button>
                      <button class="delete-btn" onClick={() => handleDelete(server.id)}>
                        Delete
                      </button>
                    </div>
                  </div>
                )}
              </For>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default MCPSettings;
