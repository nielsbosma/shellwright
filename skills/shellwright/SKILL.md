# Shellwright — Agent Skill

> Universal CLI Session Broker for AI Agents.
> Use this when you need to interact with **interactive terminal programs** — prompts, REPLs, TUI apps, password inputs — that a normal shell command cannot handle.

## Install

```bash
# Option 1: npm (no Rust needed)
npm install -g shellwright

# Option 2: Cargo
cargo install shellwright

# Option 3: npx (zero install, runs directly)
npx shellwright --help
```

The daemon (`shellwrightd`) auto-starts on first use. No manual setup needed.

## When to Use

Use Shellwright when a command **asks for input** and would hang or timeout in a normal shell:

- Confirmation prompts (`[Y/n]`, `Enter a value:`)
- Password / secret prompts
- Interactive wizards (`npm init`, `dotnet new`)
- REPLs (`python`, `node`, `psql`, `redis-cli`)
- Selection menus (arrow keys + Enter)
- TUI apps (`git rebase -i`, `terraform apply`)

**Do NOT use** for non-interactive commands — just run those normally.

## Commands

| Command | Purpose | Example |
|---|---|---|
| `start` | Launch interactive session | `shellwright start --name tf -- terraform apply` |
| `read` | Read session output | `shellwright read tf --tail 10` |
| `send` | Send text + Enter | `shellwright send tf "yes"` |
| `wait` | Wait for pattern in output | `shellwright wait tf --for "Apply complete" --timeout 60` |
| `status` | Check session state + prompt | `shellwright status tf` |
| `list` | List all sessions | `shellwright list` |
| `interrupt` | Send Ctrl+C | `shellwright interrupt tf` |
| `terminate` | Kill session | `shellwright terminate tf` |

All commands return **JSON** by default.

## Interaction Flow

Always follow this pattern:

```
1. START the session
2. WAIT for the prompt to appear
3. SEND the input
4. WAIT for the next prompt or completion
5. Repeat 3-4 until done
6. TERMINATE the session
```

**Never send input without waiting for the prompt first.** Blind sends race against the process and may arrive before it's ready.

## Examples

### Confirm a prompt

```bash
shellwright start --name deploy -- terraform apply
shellwright wait deploy --for "Enter a value" --timeout 60
shellwright send deploy "yes"
shellwright wait deploy --for "Apply complete" --timeout 300
shellwright read deploy --tail 5
shellwright terminate deploy
```

### Drive an interactive wizard

```bash
shellwright start --name init -- npm init
shellwright wait init --for "package name" --timeout 10
shellwright send init "my-app"
shellwright wait init --for "version" --timeout 5
shellwright send init "1.0.0"
shellwright wait init --for "description" --timeout 5
shellwright send init "My application"
# ... continue for each prompt
shellwright wait init --for "Is this OK" --timeout 5
shellwright send init "yes"
shellwright terminate init
```

### REPL session

```bash
shellwright start --name py -- python
shellwright wait py --for ">>>" --timeout 5
shellwright send py "2 + 2"
shellwright wait py --for ">>>" --timeout 5
shellwright read py --tail 3
# Output includes "4"
shellwright send py "exit()"
```

### Password prompt

```bash
shellwright start --name db -- psql -h localhost -U admin mydb
shellwright wait db --for "Password" --timeout 10
shellwright send db "secret123"
shellwright wait db --for "mydb=>" --timeout 5
shellwright send db "SELECT count(*) FROM users;"
shellwright wait db --for "mydb=>" --timeout 10
shellwright read db --tail 5
shellwright terminate db
```

### Arrow key navigation (selection menus)

```bash
shellwright start --name setup -- npx create-next-app
shellwright wait setup --for "What is your project" --timeout 10
shellwright send setup "my-app"
# For selection prompts, send arrow keys:
shellwright wait setup --for "Would you like" --timeout 5
shellwright send setup $'\x1b[B'     # Down arrow
shellwright send setup ""            # Enter (confirm selection)
```

### Send + wait in one call

```bash
shellwright send deploy "yes" --wait-for "complete" --timeout 60
```

## Key Sequences

| Key | How to send |
|---|---|
| Enter | Automatic with `send` (just send the text) |
| Ctrl+C | `shellwright interrupt <session>` |
| Down arrow | `shellwright send <session> $'\x1b[B'` |
| Up arrow | `shellwright send <session> $'\x1b[A'` |
| Tab | `shellwright send <session> $'\t'` |
| Space (toggle) | `shellwright send <session> " "` |
| Escape | `shellwright send <session> $'\x1b'` |

## Reading Output

```bash
# Last N lines (most useful — saves tokens)
shellwright read build --tail 10

# Since a cursor position (for incremental reads)
shellwright read build --since 42

# Full output
shellwright read build
```

The `cursor` field in the response lets you do incremental reads:
1. First `read` returns `cursor: 42`
2. Next `read --since 42` returns only new lines + `cursor: 67`
3. Next `read --since 67` returns only newer lines

## Prompt Detection

`shellwright status` tells you if the process is waiting for input:

```json
{
  "state": "awaiting_input",
  "prompt_confidence": 0.95,
  "prompt_text": "Password: "
}
```

- **confidence >= 0.8** — safe to send input
- **confidence 0.5-0.8** — verify `prompt_text` matches expectations
- **confidence < 0.5** — process likely still running, wait longer

## Dangerous Commands

Shellwright blocks dangerous commands (`rm -rf`, `DROP TABLE`, `git push --force`, etc.) by default. To override:

```bash
shellwright confirm-danger "rm -rf /tmp/data" "Cleaning CI artifacts from failed run"
shellwright start --name cleanup -- rm -rf /tmp/data
```

## Session Lifecycle

- Sessions persist across agent invocations (daemon-backed)
- Idle sessions are cleaned up after 30 minutes
- The daemon self-exits after 10 minutes with no sessions
- Named sessions: use `--name` for clarity, or let it auto-generate
