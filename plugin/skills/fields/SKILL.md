---
name: fields
description: Inspect Jira field metadata with jirac for a project and issue type, especially before create or update flows that use custom fields or backlog-specific schemas
---

List available Jira fields using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - project key
   - optional issue type
   - whether they want only required fields
3. Run one of these:
   - `jirac issue fields -p <PROJECT>`
   - `jirac issue fields -p <PROJECT> --issue-type '<TYPE>'`
   - add `--required-only` when requested
4. Highlight the field IDs and likely value shapes that matter for the next step.

Examples:
- "show required fields for Story in PROJ" → `jirac issue fields -p PROJ --issue-type 'Story' --required-only`
- "what custom fields are available in PROJ" → `jirac issue fields -p PROJ`
