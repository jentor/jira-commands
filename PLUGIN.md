# Jira Agent Plugin

The `plugin/` package supplies Jira CLI skills in the portable [Agent Plugins 1.0.0](https://agent-plugins.org/specification) format. Codex and Cursor load the root manifest; Claude Code continues to use the compatibility manifest and `/jira:*` namespace.

The plugin has its own version under `plugin/VERSION`. ClawHub packaging and release flow remain separate.

## Setup

Install `jira-commands`, run `jirac auth login`, then follow the [client-specific installation instructions](plugin/README.md#installation). Only `jirac` must be on the client's `PATH`. MCP is an optional, separately installed integration; the plugin does not start it.

## Available skills

| Skill                   | Description                             |
| ----------------------- | --------------------------------------- |
| `list-issues`     | List issues by project or JQL           |
| `view-issue`      | View full issue detail                  |
| `checklist`       | Manage Smart Checklist items, history, and templates (runtime opt-in) |
| `create-issue`    | Create a new issue                      |
| `update-issue`    | Update an existing issue                |
| `transition`      | Transition an issue                     |
| `comment`         | List comments or add a Markdown comment |
| `worklog`         | List, add, or delete worklogs           |
| `fields`          | Inspect available field metadata        |
| `bulk-transition` | Bulk transition issues via JQL          |
| `attach`          | Upload a file to an issue               |
| `jql`             | Build and run a JQL query               |
| `api`             | Raw REST API passthrough                |
