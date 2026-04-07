const MCP_TOOL_PATTERN =
  /^mcp_([0-9a-f]{8}(?:_[0-9a-f]{4}){3}_[0-9a-f]{12})_(.+)$/i;

function humanizeToolName(value: string): string {
  return value
    .replace(/[._/:-]+/g, " ")
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(" ")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function formatToolDisplayName(toolName: string): string {
  return formatToolDisplayNameWithServers(toolName, {});
}

export function formatToolDisplayNameWithServers(
  toolName: string,
  serverNames: Record<string, string>
): string {
  const mcpMatch = toolName.match(MCP_TOOL_PATTERN);
  if (mcpMatch) {
    const serverId = mcpMatch[1].replace(/_/g, "-");
    const serverName = serverNames[serverId] || "MCP";
    return `${serverName}: ${humanizeToolName(mcpMatch[2])}`;
  }

  if (toolName.startsWith("docker_")) {
    return `Docker: ${humanizeToolName(toolName.slice("docker_".length))}`;
  }

  return humanizeToolName(toolName);
}
