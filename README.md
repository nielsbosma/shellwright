# Shellwright

**Universal CLI Session Broker for AI Agents — Playwright for CLIs**

Shellwright is a cross-platform, PTY-backed session broker that transforms terminal interaction into a clean, machine-readable protocol for AI agents. Any agent that can shell out commands can use Shellwright to drive interactive CLIs.

## Installation

```bash
cargo install shellwright
```

Or build from source:

```bash
git clone https://github.com/shellwright/shellwright
cd shellwright
cargo build --release
```

## Quick Start

```bash
# Start a session
shellwright start --name build -- npm run build

# Read output
shellwright read build

# Send input to a prompt
shellwright send build "y"

# Wait for a pattern in output
shellwright wait build --for "PASS|FAIL" --timeout 30

# List all sessions
shellwright list

# Get session status
shellwright status build

# Send Ctrl+C
shellwright interrupt build

# Terminate a session
shellwright terminate build
```

## JSON Output

Every command outputs JSON by default, making it trivially parseable by agents:

```json
{
  "id": "a1b2c3",
  "success": true,
  "data": {
    "session": "build",
    "state": "exited",
    "exit_code": 0,
    "output_file": "/tmp/shellwright/sessions/build/output.txt",
    "output_tail": "Build completed successfully",
    "lines": 847
  }
}
```

Use `--no-json` for human-readable plain text output.

## Architecture

```mermaid
graph TB
    Agent["AI Agent<br/><i>Claude, Codex, Gemini, etc.</i>"]
    CLI["shellwright<br/><i>CLI binary</i>"]
    IPC{{"IPC<br/><small>Unix socket / Named pipe</small>"}}
    Daemon["shellwrightd<br/><i>daemon</i>"]
    SM["Session Manager<br/><small>lifecycle · naming · cleanup</small>"]

    Agent -- "shell out" --> CLI
    CLI -- "JSON over IPC" --> IPC
    IPC --> Daemon
    Daemon --> SM

    subgraph Sessions ["PTY Sessions (x N concurrent)"]
        direction TB
        S1["PTY Runner<br/><small>portable-pty · ConPTY · Unix PTY</small>"]
        S2["VT Parser<br/><small>vt100 · ANSI strip · screen state</small>"]
        S3["Output Ring<br/><small>bounded buffer · cursor reads</small>"]
        S4["Prompt Detector<br/><small>15 patterns · calibration · settle</small>"]
        S5["Transcript<br/><small>append-only disk log</small>"]
        S6["Security<br/><small>danger detection · secret redaction</small>"]

        S1 --> S2 --> S3
        S2 --> S4
        S3 --> S5
        S1 --> S6
    end

    SM --> Sessions

    style Agent fill:#4A90D9,color:#fff
    style CLI fill:#2D2D2D,color:#fff
    style Daemon fill:#2D2D2D,color:#fff
    style IPC fill:#F5A623,color:#fff
    style SM fill:#7B68EE,color:#fff
    style Sessions fill:#1a1a2e,color:#fff
    style S1 fill:#333,color:#fff
    style S2 fill:#333,color:#fff
    style S3 fill:#333,color:#fff
    style S4 fill:#333,color:#fff
    style S5 fill:#333,color:#fff
    style S6 fill:#333,color:#fff
```

The daemon (`shellwrightd`) auto-starts on first `shellwright` command. Sessions persist across agent restarts. Orphaned sessions are cleaned up after a configurable timeout. The daemon self-exits after 10 minutes with no active sessions.

## Key Features

- **Agent-agnostic**: Works with any agent that can run shell commands
- **Cross-platform**: First-class Windows ConPTY + Unix PTY support
- **Daemon-backed sessions**: Persist across agent crashes and context resets
- **Token-efficient output**: Filesystem-first (file paths + tail), cursor-based reads
- **Prompt detection**: Heuristic + pattern matching with confidence scores
- **Wait-for patterns**: `shellwright wait build --for "PASS|FAIL"` — the `waitForSelector` of CLIs
- **Security**: Dangerous command detection, secret redaction
- **Clean output**: VT terminal emulation strips ANSI noise, collapses progress bars

## Comparison

| Tool | Approach | Limitation |
|---|---|---|
| expect/pexpect | Regex-based PTY scripting | Not structured, no agent protocol |
| mcp-interactive-terminal | Node.js MCP server | Node dependency, Unix-only, MCP-locked |
| Codex CLI (exec_command) | Model-driven PTY sessions | Locked to OpenAI ecosystem |
| Gemini CLI (node-pty) | User-driven embedded terminal | User must interact, not programmatic |
| **Shellwright** | **Standalone CLI session broker** | **Production-grade, cross-platform, agent-agnostic** |

## License

MIT
