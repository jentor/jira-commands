---
name: attach
description: Upload local files as Jira issue attachments through the jirac CLI
---

Upload a file attachment to a Jira issue using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - issue key, for example `PROJ-123`
   - file path
3. If the issue key or file path is missing, ask for it.
4. Verify the file exists before running the command.
5. Run `jirac issue attach <ISSUE-KEY> <FILE-PATH>`.
6. Confirm the upload result clearly.

Examples:
- "attach screenshot.png to PROJ-123" → `jirac issue attach PROJ-123 ./screenshot.png`
- "upload error.log to PROJ-456" → `jirac issue attach PROJ-456 ./error.log`
