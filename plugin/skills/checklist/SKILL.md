---
name: checklist
description: View or edit Smart Checklist Data Center items, read checklist history, and manage global or project templates with jirac when enabled in the active profile
---

Use the native `jirac checklist` commands for Smart Checklist Data Center.

## Availability and targeting

- Check `jirac --version`, `jirac checklist --help`, and `jirac config show`. If the CLI is missing, install with `cargo install jira-commands`; if the command is missing, upgrade the CLI.
- The integration is disabled by default. It requires `smart_checklist_enabled = true` in the active profile. If disabled, explain `jirac config set smart_checklist_enabled true` and stop the checklist operation. Change this setting only when the user requests enabling it.
- Uses the active Jira profile and its PAT/Basic authentication; select another existing profile with `JIRA_PROFILE` when requested.
- The public API works only with the **Default Checklist tab**. It cannot target additional tabs. History requires Smart Checklist 6.5.0+.
- Read `jirac checklist view PROJ-123 --json` before modifying existing items. Resolve item IDs and status IDs from the server response; do not guess IDs or hardcode a status-name mapping.
- Mutations resolve the checklist ID from the issue key. If several checklists are returned, specify the intended `--checklist-id` instead of silently choosing one.

## Issue operations

```bash
jirac checklist view PROJ-123
jirac checklist view PROJ-123 --json
jirac issue view PROJ-123 --checklist --json
jirac checklist history PROJ-123 --json
jirac checklist append PROJ-123 --text '- Review the release notes @developer'
jirac checklist append PROJ-123 --file checklist.txt
jirac checklist update PROJ-123 102 --label 'Review release notes' --mandatory true
jirac checklist update PROJ-123 102 --status-id <STATUS_ID>
jirac checklist update PROJ-123 --file updates.json
```

`update --file` expects an array in the plugin JSON format; use IDs discovered from the server:

```json
[{"id":102,"label":"Review release notes","mandatory":true},{"id":101,"rank":0,"level":2}]
```

Text files are UTF-8; `--file -` reads stdin. Pass Smart Checklist text unchanged, including heading prefixes, item status markers, mentions, and `>` explanations. Do not convert it to ADF or Jira Wiki markup.
Use a file for multiline content. All operations support `--json`; stdout is the result, stderr carries failures.

Replacing or clearing a checklist affects **every item**, not only a selected item:

```bash
jirac checklist replace PROJ-123 --file checklist.txt --force
jirac checklist clear PROJ-123 --force
```

Use `--force` only when the user's request already authorizes replacing or removing the whole checklist.
Without `--force`, these commands prompt on a terminal and fail in non-interactive mode.
The public API has no native per-item delete command; do not silently emulate one by rewriting the whole checklist.

## Templates

```bash
jirac checklist template list --query Release --json
jirac checklist template list --project-id 10000 --json
jirac checklist template list --project-id 10000 --global --json
jirac checklist template view 2 --json
jirac checklist template fields --json
jirac checklist template create --file template.json --json
jirac checklist template update 2 --file template.json --json
jirac checklist template apply PROJ-123 2 --json
jirac checklist template delete 2 --project-id 10000 --force --json
```

Without `--page`, listings collect all pages with a 500-page limit. `--page N` fetches one page.
Sorting uses `--order-by name|enabled|issueTypes|projects` and optional `--reversed`.
For a project, `--global` lists applicable global templates; otherwise it lists project-local templates.
Discover the intended project/template IDs before writing. Deleting a template requires an authorized deletion request before using `--force`.

Create/update input uses the plugin JSON format, with `name` and `scope` required:

```json
{
  "name": "Release review",
  "value": "# Preparation\n- Review release notes\n- Run checks\n",
  "scope": {"type": 2, "values": ["10000"]},
  "projectId": 10000,
  "enabled": true,
  "conditions": [],
  "trigger": {"type": 3, "preventDuplicates": false}
}
```

Use `template fields --json` when configuring conditions; field IDs must come from the instance.
Keep requested conditions, triggers, and project scope explicit. Treat changes to template automation as changes to future issue behavior.
Report plugin failures with their original details. Permission failures can appear as HTTP 500; do not retry with elevated credentials or fall back to editing a custom field.

## MCP alternative

When using `jirac-mcp`, the equivalent tools are `jira_checklist_*`; they are advertised only when the runtime flag is enabled at server startup. Restart the server after changing it.
`jira_issue_view` accepts `include_checklist: true`. MCP item updates use `status_id`, whereas CLI JSON files use `status: {"id": ...}`.
Destructive checklist tools require `force: true` and the same user authorization as the CLI.
