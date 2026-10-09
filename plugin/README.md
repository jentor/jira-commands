# jira — Agent Plugin
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/12742/badge)](https://www.bestpractices.dev/projects/12742)

Manage Jira issues from Codex, Cursor, Claude Code, and other Agent Plugins clients. Create, list, view, transition, comment, log time, and run bulk operations without leaving your editor.

This plugin is versioned and released independently from the main CLI/MCP workspace. See `plugin/VERSION` and `plugin/CHANGELOG.md` for the plugin release lane.

> **jirac** is an independent agent plugin for the Jira ecosystem. Not affiliated with or endorsed by Atlassian.

> **Requires** the `jirac` CLI to be installed: `cargo install jira-commands` (crates.io) or `cargo install --path crates/jira --locked` from a local checkout.
>
> You can also install `jirac` from Homebrew (`brew install mulhamna/tap/jira-commands`), npm (`npm install -g @mulham28/jirac`), the workspace shell installer on macOS/Linux, the PowerShell installer on Windows, or GitHub Releases.
>
> **Compatibility note:** the next release requires `jirac`. If you still have old scripts or aliases that call `jira`, update them before upgrading.

> The portable package follows [Agent Plugins 1.0.0](https://agent-plugins.org/specification): `plugin.json` and `skills/`. The Claude compatibility manifest remains under `.claude-plugin/`.
> Skills execute `jirac` and use its existing Jira authentication and active profile. The plugin does not register or start an MCP server.

---

## Installation

Install the CLI and sign in before loading the plugin:

```bash
cargo install jira-commands
jirac auth login
jirac --version
```

The `jirac` executable must be on the client process's `PATH`. If it is missing, install it and restart the client. No MCP binary is required for the plugin.

### Codex

After the package is published to the repository:

```bash
codex plugin marketplace add mulhamna/jira-commands
```

To test an unpublished local checkout, run from its root:

```bash
codex plugin marketplace add .
codex plugin list --marketplace jira-commands --available --json
```

Install `jira` from the `jira-commands` source in the Plugins interface and start a new conversation. Codex versions with CLI plugin installation also support:

```bash
codex plugin add jira@jira-commands
```

The catalog lives at `.agents/plugins/marketplace.json`; `./plugin` resolves relative to the repository root. Installation is opt-in. MCP remains separate and opt-in; see the optional integration below.

See [OpenAI's packaging guide](https://developers.openai.com/plugins/build/plugins) for supported surfaces and marketplace management.

### Cursor

From a local checkout, copy the package into the local plugin directory:

```bash
mkdir -p ~/.cursor/plugins/local
cp -R plugin ~/.cursor/plugins/local/jira
```

Use a fresh destination when testing; to update an existing copy, replace its package files with the current checkout. Do not symlink to an external checkout: Cursor rejects local plugin links outside its local plugin directory.

Restart Cursor or run **Developer: Reload Window**, then open **Customize** and confirm the Jira skills. Local plugin imports must be allowed by your account or organization. See [Cursor's plugin documentation](https://cursor.com/docs/plugins).

### Claude Code

The existing marketplace installation remains supported and requires only `jirac`:

```text
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

Claude uses the compatibility manifest and CLI skills.

### Optional MCP integration

Install and register MCP only if you explicitly want it:

```bash
cargo install jira-mcp
jirac mcp install --client codex
# Or select your client:
jirac mcp install --client cursor
jirac mcp install --client claude-code
```

`plugin/examples/mcp.json` is a portable configuration example, not an auto-loaded component. To deliberately bundle MCP in your own local copy, copy that example to the plugin root as `mcp.json`, install `jirac-mcp`, and reload the client. This makes the server part of that plugin's runtime wherever the plugin is enabled. Choose either standalone registration or bundling to avoid duplicate tool sets.

## Skills

Skill invocation depends on the client. Claude Code exposes these names as `/jira:<skill-name>`; other clients use their skill picker or automatic discovery.

| Skill | Description |
|---|---|
| `list-issues` | List issues by project, assignee, or custom JQL |
| `view-issue` | View full issue detail — description, status, assignee, attachments |
| `checklist` | Manage Smart Checklist Data Center items, history, and templates (runtime opt-in) |
| `create-issue` | Create a new issue with interactive field prompts |
| `update-issue` | Update summary, description, assignee, labels, versions, or custom fields |
| `transition` | Move an issue to a new status (e.g. In Progress, Done) |
| `comment` | List comments or add a new Markdown comment on an issue |
| `worklog` | List, add, or delete time entries on an issue |
| `batch` | Run mixed Jira operations from a JSON manifest |
| `bulk-create` | Create issues from a JSON manifest |
| `bulk-update` | Update assignee or priority for issues matching JQL |
| `clone-issue` | Clone an issue into the same or another project |
| `delete-issue` | Permanently delete an issue when explicitly authorized |
| `watch` | List, add, or remove issue watchers |
| `bulk-comment` | Add one Markdown comment to many issues via JQL or explicit keys |
| `bulk-transition` | Transition multiple issues at once via JQL query |
| `attach` | Upload a file or image to an issue |
| `fields` | Inspect available Jira fields for a project and issue type |
| `jql` | Build and run a JQL query interactively |
| `daily-standup` | Generate a markdown-ready daily standup summary from assigned Jira issues |
| `sprint-summary` | Summarize the current or named sprint by status and assignee |
| `sprint-lifecycle` | List, create, start, complete, update, or delete sprints on a project board |
| `notifications` | Scan recent Jira `@mention` events from descriptions and comments |
| `versions` | Browse project fix versions, backlog previews, and version metadata |
| `render` | Preview Markdown → Jira ADF conversion before posting descriptions or comments |
| `link` | Manage issue links (Blocks, Relates, Duplicates, etc.) |
| `change-type` | Change an issue's type within the same project (native Jira move semantics) |
| `move-issue` | Move an issue across projects (native Jira move; history preserved, key changes) |
| `archive` | Archive issues matching a JQL query (destructive — confirm first) |
| `api` | Execute any raw Jira REST API call (GET, POST, PUT, DELETE, PATCH) |

---

## Use cases

**Daily standup prep**
> "generate my standup for today" → `daily-standup`

The agent groups your assigned issues into recently done, in progress, next up, and blocked buckets.

---

**Create a bug report from a stack trace**
> "create a bug in PROJ for this null pointer exception"

The agent runs `create-issue`, sets type to Bug, and uses the stack trace as the description.

---

**Transition after a PR merge**
> "mark PROJ-123 as done"

The agent runs `transition` and moves the issue to Done in one step.

---

**Leave a follow-up comment**
> "comment on PROJ-456 that QA verified the fix in staging"

The agent runs `comment` and adds the requested Markdown comment.

---

**Log time at end of day**
> "log 3 hours on PROJ-456 for implementing the login flow"

The agent runs `worklog` with `--time 3h` and the comment filled in.

---

**Bulk close resolved issues**
> "close all done issues in PROJ that haven't been updated in 30 days"

The agent builds the JQL and runs `bulk-transition` with `--force`.

---

**Bulk status reminder**
> "comment on all sprint issues asking owners to post an update"

The agent can use `bulk-comment` with a JQL query or an explicit issue-key list.

---

**Explore the API**
> "get the field schema for project PROJ"

The agent calls `api` with the appropriate REST endpoint and shows the raw JSON.

---

## Configuration

Credentials are stored at `~/.config/jira/config.toml` after running `jirac auth login`. The plugin reads from the same config — no extra setup needed.

Smart Checklist Data Center support is disabled by default. Enable it for the active profile with
`jirac config set smart_checklist_enabled true`, or add `smart_checklist_enabled = true` under that profile in TOML.
The `checklist` skill uses the Default Checklist tab only. Restart the MCP server after changing this flag so its advertised tools refresh.

You can also use environment variables:

```bash
export JIRA_URL=https://yourcompany.atlassian.net
export JIRA_EMAIL=you@example.com
export JIRA_TOKEN=your_api_token
```

---

## Source

[github.com/mulhamna/jira-commands](https://github.com/mulhamna/jira-commands)


**Sprint rollup**
> "summarize the current sprint in PROJ" → `sprint-summary`

The agent summarizes sprint progress by status and assignee for a quick project pulse.
