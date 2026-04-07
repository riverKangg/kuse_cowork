import { Component, For, Show, createSignal, onMount } from "solid-js";
import { useSettings } from "../stores/settings";
import { runAgent, AgentEvent, isTauri } from "../lib/tauri-api";
import { getMCPServerStatuses } from "../lib/mcp-api";
import { formatToolDisplayNameWithServers } from "../lib/tool-display";
import "./Agent.css";

interface ToolExecution {
  id: number;
  tool: string;
  input: Record<string, unknown>;
  result?: string;
  success?: boolean;
  status: "running" | "completed" | "error";
}

const Agent: Component = () => {
  const { isConfigured, toggleSettings } = useSettings();

  const [input, setInput] = createSignal("");
  const [projectPath, setProjectPath] = createSignal("");
  const [isRunning, setIsRunning] = createSignal(false);
  const [currentText, setCurrentText] = createSignal("");
  const [toolExecutions, setToolExecutions] = createSignal<ToolExecution[]>([]);
  const [currentTurn, setCurrentTurn] = createSignal(0);
  const [totalTurns, setTotalTurns] = createSignal<number | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [serverNames, setServerNames] = createSignal<Record<string, string>>({});

  let outputEnd: HTMLDivElement | undefined;

  onMount(async () => {
    try {
      const statuses = await getMCPServerStatuses();
      setServerNames(
        Object.fromEntries(statuses.map((status) => [status.id, status.name]))
      );
    } catch (loadError) {
      console.error("Failed to load MCP server names:", loadError);
    }
  });

  const scrollToBottom = () => {
    outputEnd?.scrollIntoView({ behavior: "smooth" });
  };

  const handleEvent = (event: AgentEvent) => {
    console.log("Agent event:", event);
    switch (event.type) {
      case "text":
        setCurrentText(event.content);
        scrollToBottom();
        break;
      case "tool_start":
        setToolExecutions((prev) => [
          ...prev,
          {
            id: Date.now(),
            tool: event.tool,
            input: event.input,
            status: "running",
          },
        ]);
        scrollToBottom();
        break;
      case "tool_end":
        setToolExecutions((prev) => {
          const updated = [...prev];
          const last = updated.findLast((t: any) => t.tool === event.tool && t.status === "running");
          if (last) {
            last.result = event.result;
            last.success = event.success;
            last.status = event.success ? "completed" : "error";
          }
          return updated;
        });
        scrollToBottom();
        break;
      case "turn_complete":
        setCurrentTurn(event.turn);
        break;
      case "done":
        setTotalTurns(event.total_turns);
        setIsRunning(false);
        break;
      case "error":
        setError(event.message);
        setIsRunning(false);
        break;
    }
  };

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    const message = input().trim();
    if (!message || isRunning()) return;

    // Reset state
    setInput("");
    setCurrentText("");
    setToolExecutions([]);
    setCurrentTurn(0);
    setTotalTurns(null);
    setError(null);
    setIsRunning(true);

    try {
      await runAgent(
        {
          message,
          project_path: projectPath() || undefined,
          max_turns: 100,
        },
        handleEvent
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
      setIsRunning(false);
    }
  };

  const formatInput = (input: Record<string, unknown>): string => {
    const entries = Object.entries(input);
    if (entries.length === 0) return "";
    return entries
      .map(([key, value]) => {
        const strValue = typeof value === "string" ? value : JSON.stringify(value);
        const truncated = strValue.length > 100 ? strValue.slice(0, 100) + "..." : strValue;
        return `${key}: ${truncated}`;
      })
      .join(", ");
  };

  return (
    <div class="agent">
      <Show
        when={isConfigured()}
        fallback={
          <div class="agent-setup">
            <h2>Agent Mode</h2>
            <p>Configure your API key to use the agent</p>
            <button onClick={toggleSettings}>Open Settings</button>
          </div>
        }
      >
        <Show
          when={isTauri()}
          fallback={
            <div class="agent-setup">
              <h2>Agent Mode</h2>
              <p>Agent mode requires the desktop app to execute tools</p>
            </div>
          }
        >
          <div class="agent-output">
            <Show when={toolExecutions().length > 0 || currentText()}>
              <div class="output-section">
                <Show when={currentText()}>
                  <div class="agent-text">
                    <div class="text-label">Claude</div>
                    <div class="text-content">{currentText()}</div>
                  </div>
                </Show>

                <Show when={toolExecutions().length > 0}>
                  <div class="tools-section">
                    <div class="tools-label">Tool Executions</div>
                    <For each={toolExecutions()}>
                      {(tool) => (
                        <div class={`tool-execution ${tool.status}`}>
                          <div class="tool-header">
                            <span class="tool-name" title={tool.tool}>
                              {formatToolDisplayNameWithServers(tool.tool, serverNames())}
                            </span>
                            <span class={`tool-status ${tool.status}`}>
                              {tool.status === "running" && "Running..."}
                              {tool.status === "completed" && "Done"}
                              {tool.status === "error" && "Error"}
                            </span>
                          </div>
                          <div class="tool-input">
                            <span class="input-label">Input:</span> {formatInput(tool.input)}
                          </div>
                          <Show when={tool.result}>
                            <div class="tool-result">
                              <span class="result-label">Result:</span>
                              <pre class="result-content">
                                {tool.result!.length > 500
                                  ? tool.result!.slice(0, 500) + "..."
                                  : tool.result}
                              </pre>
                            </div>
                          </Show>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>

                <Show when={totalTurns() !== null}>
                  <div class="completion-status">
                    Completed in {totalTurns()} turn{totalTurns() !== 1 ? "s" : ""}
                  </div>
                </Show>

                <Show when={error()}>
                  <div class="error-status">{error()}</div>
                </Show>
              </div>
            </Show>

            <Show when={!toolExecutions().length && !currentText() && !isRunning()}>
              <div class="empty-agent">
                <h2>Agent Mode</h2>
                <p>
                  The agent can execute tools to help you with tasks like reading files, running
                  commands, and searching code.
                </p>
                <p class="tools-list">
                  Available tools: file_read, file_write, file_edit, bash, glob, grep, list_dir
                </p>
              </div>
            </Show>

            <div ref={outputEnd} />
          </div>

          <div class="agent-input-section">
            <div class="project-path-input">
              <label for="project-path">Project Path (optional):</label>
              <input
                id="project-path"
                type="text"
                value={projectPath()}
                onInput={(e) => setProjectPath(e.currentTarget.value)}
                placeholder="/path/to/project"
                disabled={isRunning()}
              />
            </div>

            <form class="agent-form" onSubmit={handleSubmit}>
              <textarea
                value={input()}
                onInput={(e) => setInput(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    handleSubmit(e);
                  }
                }}
                placeholder="Ask the agent to help you... (e.g., 'Read the README.md file')"
                disabled={isRunning()}
                rows={3}
              />
              <button type="submit" disabled={isRunning() || !input().trim()}>
                {isRunning() ? `Running (Turn ${currentTurn()})...` : "Run Agent"}
              </button>
            </form>
          </div>
        </Show>
      </Show>
    </div>
  );
};

export default Agent;
