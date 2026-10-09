# jira-mcp

> **jirac-mcp** exposes Jira operations over the Model Context Protocol (MCP).
> It is **not** affiliated with, endorsed by, or sponsored by Atlassian.

`jira-mcp` is the MCP server crate in the `mulhamna/jira-commands` workspace. It reuses `jira-core` and exposes typed Jira tools for editors, assistants, and remote MCP clients.

[![License: MIT%20OR%20Apache--2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/12742/badge)](https://www.bestpractices.dev/projects/12742)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](http://makeapullrequest.com)

## Install

```bash
# Homebrew (macOS / Linux)
brew tap mulhamna/tap
brew install jira-mcp

# Cargo (crates.io)
cargo install jira-mcp

# npm (Node 18+)
npm install -g @mulham28/jirac-mcp

# Scoop (Windows)
scoop bucket add mulhamna https://github.com/mulhamna/scoop-bucket
scoop install mulhamna/jirac-mcp

# From a local checkout
cargo install --path crates/jira-mcp --locked
```

You can also use the workspace shell installer on macOS/Linux, the PowerShell installer flow on Windows (x64 + ARM64), or download packaged release archives from GitHub Releases. Native Windows ARM64 binaries ship as `jirac-mcp-windows-aarch64.zip`.

## Run

```bash
# Local stdio transport
jirac-mcp serve --transport stdio

# Streamable HTTP transport
jirac-mcp serve --transport streamable-http --host 127.0.0.1 --port 8787 --path /mcp
```

## Shared configuration

The server reuses the same credentials/config as `jirac`:
- `~/.config/jira/config.toml`
- `JIRA_PROFILE`
- `JIRA_URL`
- `JIRA_EMAIL`
- `JIRA_TOKEN`

You can initialize and switch credentials with:

```bash
jirac auth login
jirac auth profiles
jirac auth use work-cloud
```

## Tool coverage

The MCP server includes tools for:
- auth status and credential updates
- diagnostics: `jira_whoami` (current account id, timezone, base URL) and `jira_server_info` (server info + premium/Plans detection)
- issue list, view, create, update, delete, clone, move (native cross-project move, `confirm: true` required), batch flows, standups, sprint summaries, and notifications
- notifications: mark scanned notifications as read by id (`jira_notifications_mark_read`)
- field and transition discovery
- comments (single + bulk)
- attachments: upload, list, download (writes to disk with `$HOME` guard and `force_path` opt-out), and delete (`confirm: true` required)
- watchers: list, add (defaults to the current authenticated user via `/myself`), and remove (`confirm: true` required)
- worklog operations
- structured JQL builder: `jira_jql_build` composes safe JQL from a `JqlParams` object, with optional `dry_run` preview of matching issue keys
- Agile boards: list (project + type filter), get, list issues (JQL + max), and list backlog
- bulk transition, bulk update, and archive flows
- plans
- raw Jira REST API requests
- Smart Checklist Data Center: checklist items, history, and global/project templates

### Smart Checklist Data Center

The public Railsware API addresses only the **Default Checklist tab** and uses the active Jira profile.
History requires plugin v6.5.0+. No TUI or private plugin endpoints are used.
This integration is disabled by default. Set `smart_checklist_enabled = true` in the active profile's TOML configuration,
or run `jirac config set smart_checklist_enabled true`, then restart `jirac-mcp`.
When disabled, no `jira_checklist_*` tools are advertised, and `jira_issue_view` rejects `include_checklist: true` before making a Jira request.

| Tools | Arguments |
| --- | --- |
| `jira_checklist_view` | `issue_key`, optional `checklist_id` |
| `jira_checklist_history` | `issue_key` |
| `jira_checklist_append` | `issue_key`, `text`, optional `checklist_id` |
| `jira_checklist_replace` | Same as append, plus `force: true` |
| `jira_checklist_update` | `issue_key`, `updates`, optional `checklist_id` |
| `jira_checklist_clear` | `issue_key`, optional `checklist_id`, `force: true` |
| `jira_checklist_template_list` | Optional `project_id`, `global`, `query`, `order_by`, `reversed`, `page` |
| `jira_checklist_template_view` | `template_id` |
| `jira_checklist_template_fields` | No arguments |
| `jira_checklist_template_create` | `template` object in the plugin JSON format |
| `jira_checklist_template_update` | `template_id`, `template` object |
| `jira_checklist_template_delete` | `template_id`, optional `project_id`, `page`, `force: true` |
| `jira_checklist_template_apply` | `issue_key`, `template_id`, optional `checklist_id` |

`updates` is an array of `{id, label?, status_id?, rank?, level?, mandatory?}`.
Use server item/status IDs from checklist responses, not fixed status-name mappings.
Text is passed unchanged, preserving the plugin's heading/list/mention/explanation syntax.
Template JSON requires `name` and `scope`; it can include `value`, `enabled`, `conditions`, `trigger`, and `projectId`.
Omitting `page` collects all template pages (maximum 500). With `project_id`, `global: true` selects applicable global templates;
otherwise project-local templates are listed. Multiple returned checklists require `checklist_id` for mutations.

`jira_issue_view` accepts optional `include_checklist: true` and adds a `checklists` array to the existing issue result.
Without it, no plugin request is made. An explicitly requested checklist failure is returned as an error.

## Notes

- Current focus is tools, not prompts/resources/UI.
- Destructive operations require `confirm: true`; destructive `jira_checklist_*` tools require `force: true`.
- `jira_issue_clone` can optionally delete the source issue with `move_original: true`, but only when `confirm: true` is also set.
- Attachment uploads support local file paths or inline base64 payloads.

## Client install helper

If you already have both `jirac` and `jirac-mcp` installed, register the MCP server into a supported client with:

```bash
jirac mcp doctor          # check prereqs only
jirac mcp install         # interactive picker (recommended)
```

The interactive flow verifies that `jirac-mcp` is on PATH and that Jira auth is configured, then lets you pick the client to install into. Pass `--client` explicitly to skip the picker in scripts:

```bash
jirac mcp install --client claude-code
jirac mcp install --client claude-desktop
jirac mcp install --client cursor
jirac mcp install --client gemini-cli
jirac mcp install --client codex
jirac mcp install --client opencode
jirac mcp install --client generic-json
```

Notes:
- `claude-code` writes user-level `~/.claude.json` (`mcpServers`); project-scope `.mcp.json` is not written by this helper
- `claude-desktop` writes the platform support dir (`claude_desktop_config.json`) — macOS Library, Windows APPDATA, Linux XDG
- `gemini-cli` and `codex` delegate to their native CLI `mcp add` flows; `opencode` writes `~/.config/opencode/opencode.jsonc` directly
- `generic-json` prints a portable JSON snippet instead of writing a file
- `cursor` remains provisional until verified in a real Cursor install

## More docs

See the root README and `INSTALL.md` for client-specific install notes, helper target details, and workspace-level context.


## Release notes

This package is released from the `crates/jira-mcp` lane. User-facing fixes in MCP docs, request handling, or package metadata should use a releasable Conventional Commit type such as `fix(jira-mcp): ...` when they need a patch release.

