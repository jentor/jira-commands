use std::{
    fmt::Write as _,
    io::{IsTerminal, Read},
    path::PathBuf,
};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use jira_core::{model::checklist::*, JiraClient};

use super::interactive::require_interactive;

#[derive(Debug, Args)]
pub struct ChecklistTarget {
    /// Jira issue key
    pub key: String,
    /// Required if the API returns more than one checklist
    #[arg(long)]
    pub checklist_id: Option<u64>,
}

#[derive(Debug, Args)]
pub struct ChecklistText {
    /// Smart Checklist text (no ADF or Wiki conversion)
    #[arg(
        long,
        required_unless_present = "file",
        conflicts_with = "file",
        allow_hyphen_values = true
    )]
    pub text: Option<String>,
    /// UTF-8 input file, or - for stdin
    #[arg(long, conflicts_with = "text")]
    pub file: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum ChecklistCommand {
    /// Show headings, items, status IDs, assignees, and explanations
    View {
        #[command(flatten)]
        target: ChecklistTarget,
        #[arg(long)]
        json: bool,
    },
    /// Show checklist changes (Smart Checklist 6.5.0+)
    History {
        key: String,
        #[arg(long)]
        json: bool,
    },
    /// Append Smart Checklist text without replacing existing items
    Append {
        #[command(flatten)]
        target: ChecklistTarget,
        #[command(flatten)]
        input: ChecklistText,
        #[arg(long)]
        json: bool,
    },
    /// Replace all items with Smart Checklist text; requires confirmation
    Replace {
        #[command(flatten)]
        target: ChecklistTarget,
        #[command(flatten)]
        input: ChecklistText,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    /// Update one item using flags, or many using a JSON array file
    Update {
        #[command(flatten)]
        target: ChecklistTarget,
        #[arg(required_unless_present = "file", conflicts_with = "file")]
        item_id: Option<u64>,
        #[arg(long, conflicts_with_all = ["item_id", "label", "status_id", "rank", "level", "mandatory"])]
        file: Option<PathBuf>,
        #[arg(long)]
        label: Option<String>,
        /// Server status ID; never a hardcoded name-to-ID mapping
        #[arg(long)]
        status_id: Option<u64>,
        #[arg(long)]
        rank: Option<u32>,
        #[arg(long)]
        level: Option<u32>,
        /// Explicit true or false
        #[arg(long)]
        mandatory: Option<bool>,
        #[arg(long)]
        json: bool,
    },
    /// Remove every item; requires confirmation
    Clear {
        #[command(flatten)]
        target: ChecklistTarget,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    /// Manage global/project templates and automation configuration
    Template {
        #[command(subcommand)]
        command: ChecklistTemplateCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ChecklistTemplateCommand {
    /// List all pages by default; --page fetches just one
    List {
        #[arg(long)]
        project_id: Option<u64>,
        /// Global templates applicable to --project-id, rather than project-local templates
        #[arg(long, requires = "project_id")]
        global: bool,
        #[arg(long)]
        query: Option<String>,
        #[arg(long, value_parser = ["name", "enabled", "issueTypes", "projects"])]
        order_by: Option<String>,
        #[arg(long)]
        reversed: bool,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    View {
        template_id: u64,
        #[arg(long)]
        json: bool,
    },
    /// Show fields available for template conditions
    Fields {
        #[arg(long)]
        json: bool,
    },
    /// Create a template using the plugin JSON format (scope/conditions/trigger)
    Create {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Update a template using the plugin JSON format; include name and scope
    Update {
        template_id: u64,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Delete a template; requires confirmation
    Delete {
        template_id: u64,
        #[arg(long)]
        project_id: Option<u64>,
        #[arg(long, default_value = "1")]
        page: u32,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    /// Apply a server template to the issue's Default Checklist
    Apply {
        #[command(flatten)]
        target: ChecklistTarget,
        template_id: u64,
        #[arg(long)]
        json: bool,
    },
}

fn read_input(path: &std::path::Path) -> Result<String> {
    if path == std::path::Path::new("-") {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .context("Failed to read stdin")?;
        Ok(text)
    } else {
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))
    }
}

fn text_input(input: ChecklistText) -> Result<String> {
    match (input.text, input.file) {
        (Some(text), None) => Ok(text),
        (None, Some(file)) => read_input(&file),
        _ => bail!("Provide exactly one of --text or --file"),
    }
}

fn confirm(message: &str, force: bool) -> Result<bool> {
    if force {
        return Ok(true);
    }
    require_interactive("confirmation", "--force")?;
    if !inquire::Confirm::new(message)
        .with_default(false)
        .prompt()
        .context("Failed to read confirmation")?
    {
        println!("Aborted.");
        return Ok(false);
    }
    Ok(true)
}

pub async fn handle(cmd: ChecklistCommand, client: JiraClient) -> Result<()> {
    client.ensure_smart_checklist_enabled()?;
    match cmd {
        ChecklistCommand::View { target, json } => {
            let mut result = client.get_smart_checklists(&target.key).await?;
            if let Some(id) = target.checklist_id {
                result.checklists.retain(|c| c.checklist_id == id);
                if result.checklists.is_empty() {
                    bail!("Checklist {id} does not belong to {}", target.key);
                }
            }
            print_checklists(&result, json)?;
        }
        ChecklistCommand::History { key, json } => {
            let events = client.get_smart_checklist_history(&key).await?;
            if json {
                print_json(&events)?;
            } else {
                for event in events {
                    println!(
                        "{}  {}  {}  {}",
                        event.id,
                        event.created_at,
                        event.category,
                        terminal_text(&event.performer_name)
                    );
                    println!("  From: {}", serde_json::to_string(&event.from)?);
                    println!("  To:   {}", serde_json::to_string(&event.to)?);
                }
            }
        }
        ChecklistCommand::Append {
            target,
            input,
            json,
        } => {
            let text = text_input(input)?;
            let id = client
                .resolve_smart_checklist(&target.key, target.checklist_id)
                .await?;
            print_checklists(&client.write_smart_checklist(id, &text, false).await?, json)?;
        }
        ChecklistCommand::Replace {
            target,
            input,
            force,
            json,
        } => {
            let text = text_input(input)?;
            let id = client
                .resolve_smart_checklist(&target.key, target.checklist_id)
                .await?;
            if !confirm(
                &format!("Replace all items in checklist {id} on {}?", target.key),
                force,
            )? {
                return Ok(());
            }
            print_checklists(&client.write_smart_checklist(id, &text, true).await?, json)?;
        }
        ChecklistCommand::Update {
            target,
            item_id,
            file,
            label,
            status_id,
            rank,
            level,
            mandatory,
            json,
        } => {
            let updates = if let Some(file) = file {
                serde_json::from_str::<Vec<SmartChecklistItemUpdate>>(&read_input(&file)?)
                    .context("Expected a JSON array of item updates")?
            } else {
                vec![SmartChecklistItemUpdate {
                    id: item_id.context("Provide ITEM_ID or --file")?,
                    label,
                    status: status_id.map(|id| SmartChecklistStatusId { id }),
                    rank,
                    level,
                    mandatory,
                }]
            };
            let id = client
                .resolve_smart_checklist(&target.key, target.checklist_id)
                .await?;
            print_checklists(&client.update_smart_checklist(id, &updates).await?, json)?;
        }
        ChecklistCommand::Clear {
            target,
            force,
            json,
        } => {
            let id = client
                .resolve_smart_checklist(&target.key, target.checklist_id)
                .await?;
            if !confirm(
                &format!("Remove every item in checklist {id} on {}?", target.key),
                force,
            )? {
                return Ok(());
            }
            print_checklists(&client.clear_smart_checklist(id).await?, json)?;
        }
        ChecklistCommand::Template { command } => handle_template(command, &client).await?,
    }
    Ok(())
}

async fn handle_template(cmd: ChecklistTemplateCommand, client: &JiraClient) -> Result<()> {
    match cmd {
        ChecklistTemplateCommand::List {
            project_id,
            global,
            query,
            order_by,
            reversed,
            page,
            json,
        } => {
            let result = client
                .list_smart_checklist_templates(&SmartChecklistTemplateQuery {
                    project_id,
                    global,
                    query,
                    order_by,
                    reversed,
                    page,
                })
                .await?;
            if json {
                print_json(&result)?;
            } else {
                for template in result.templates {
                    println!(
                        "{}  [{}] {}",
                        template.id,
                        if template.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        },
                        terminal_text(&template.name)
                    );
                }
            }
        }
        ChecklistTemplateCommand::View { template_id, json } => {
            let template = client.get_smart_checklist_template(template_id).await?;
            if json {
                print_json(&template)?;
            } else {
                println!(
                    "{} — {}\n{}",
                    template.id,
                    terminal_text(&template.name),
                    display_links(&template.value, std::io::stdout().is_terminal())
                );
            }
        }
        ChecklistTemplateCommand::Fields { json } => {
            let fields = client.get_smart_checklist_template_fields().await?;
            if json {
                print_json(&fields)?;
            } else {
                for field in fields {
                    println!(
                        "{}  {}  {}",
                        terminal_text(field["id"].as_str().unwrap_or("")),
                        terminal_text(field["name"].as_str().unwrap_or("")),
                        terminal_text(field["type"].as_str().unwrap_or(""))
                    );
                }
            }
        }
        ChecklistTemplateCommand::Create { file, json } => {
            let request: SmartChecklistTemplateRequest =
                serde_json::from_str(&read_input(&file)?).context("Invalid template JSON")?;
            let result = client.create_smart_checklist_template(&request).await?;
            if json {
                print_json(&result)?;
            } else {
                println!(
                    "✓ Created template {} — {}",
                    result.id,
                    terminal_text(&result.name)
                );
            }
        }
        ChecklistTemplateCommand::Update {
            template_id,
            file,
            json,
        } => {
            let request: SmartChecklistTemplateRequest =
                serde_json::from_str(&read_input(&file)?).context("Invalid template JSON")?;
            let result = client
                .update_smart_checklist_template(template_id, &request)
                .await?;
            if json {
                print_json(&result)?;
            } else {
                println!(
                    "✓ Updated template {} — {}",
                    result.id,
                    terminal_text(&result.name)
                );
            }
        }
        ChecklistTemplateCommand::Delete {
            template_id,
            project_id,
            page,
            force,
            json,
        } => {
            if !confirm(&format!("Delete template {template_id}?"), force)? {
                return Ok(());
            }
            let result = client
                .delete_smart_checklist_template(template_id, project_id, page)
                .await?;
            if json {
                print_json(&result)?;
            } else {
                println!("✓ Deleted template {template_id}");
            }
        }
        ChecklistTemplateCommand::Apply {
            target,
            template_id,
            json,
        } => {
            let id = client
                .resolve_smart_checklist(&target.key, target.checklist_id)
                .await?;
            let result = client
                .apply_smart_checklist_template(id, template_id)
                .await?;
            if json {
                print_json(&result)?;
            } else {
                println!(
                    "✓ Applied template {template_id} to {} (checklist {id})",
                    target.key
                );
            }
        }
    }
    Ok(())
}

fn print_json(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn print_checklists(checklists: &SmartChecklists, json: bool) -> Result<()> {
    if json {
        print_json(checklists)?;
    } else {
        print!(
            "{}",
            render_checklists(checklists, std::io::stdout().is_terminal())
        );
    }
    Ok(())
}

fn render_checklists(checklists: &SmartChecklists, links: bool) -> String {
    let mut out = String::new();
    if checklists.checklists.is_empty() {
        return "No Smart Checklist found.\n".into();
    }
    for checklist in &checklists.checklists {
        writeln!(out, "Checklist {} (Default tab)", checklist.checklist_id).unwrap();
        let mut items: Vec<_> = checklist.items.iter().collect();
        items.sort_by_key(|item| item.rank);
        for item in items {
            let label = display_links(&item.label, links);
            if item.item_type.eq_ignore_ascii_case("heading") {
                writeln!(
                    out,
                    "{} {} [item {}]",
                    "#".repeat(item.level.unwrap_or(1).clamp(1, 6) as usize),
                    label,
                    item.id
                )
                .unwrap();
            } else {
                let status = item
                    .status
                    .as_ref()
                    .map(|s| format!("{} / {}", terminal_text(&s.name), s.id))
                    .unwrap_or_else(|| "no status".into());
                writeln!(
                    out,
                    "- [{}] {} [item {}]{}",
                    status,
                    label,
                    item.id,
                    if item.mandatory { " (mandatory)" } else { "" }
                )
                .unwrap();
            }
            for assignee in &item.assignees {
                writeln!(
                    out,
                    "  @{} ({})",
                    terminal_text(&assignee.user_name),
                    terminal_text(&assignee.display_name)
                )
                .unwrap();
            }
            let mut quotes: Vec<_> = item.quotes.iter().collect();
            quotes.sort_by_key(|quote| quote.rank);
            for quote in quotes {
                let text = if quote.text.is_empty() {
                    &quote.label
                } else {
                    &quote.text
                };
                for line in display_links(text, links).lines() {
                    writeln!(out, "  > {line}").unwrap();
                }
            }
        }
    }
    out
}

// Server text must not inject terminal escape sequences into human output.
fn terminal_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

// shortcut: inline Markdown links only; use a full Markdown renderer if richer terminal formatting is needed.
fn display_links(text: &str, clickable: bool) -> String {
    let text = terminal_text(text);
    let mut rest = text.as_str();
    let mut out = String::new();
    while let Some(start) = rest.find('[') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(mid) = rest.find("](") else {
            break;
        };
        let mut depth = 0;
        let end = rest[mid + 2..].char_indices().find_map(|(i, c)| {
            match c {
                '(' => depth += 1,
                ')' if depth == 0 => return Some(i),
                ')' => depth -= 1,
                _ => {}
            }
            None
        });
        let Some(end) = end else {
            break;
        };
        let end = mid + 2 + end;
        let url = &rest[mid + 2..end];
        if url.starts_with("https://") || url.starts_with("http://") {
            let label = &rest[1..mid];
            if clickable && !url.chars().any(char::is_control) {
                write!(out, "\x1b]8;;{url}\x1b\\{label}\x1b]8;;\x1b\\").unwrap();
            } else {
                out.push_str(label);
            }
            rest = &rest[end + 1..];
        } else {
            out.push('[');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: ChecklistCommand,
    }

    #[test]
    fn native_command_inputs_are_unambiguous() {
        let cli = TestCli::try_parse_from([
            "checklist",
            "append",
            "PROJ-1",
            "--text",
            "- Item @developer",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            ChecklistCommand::Append {
                input: ChecklistText {
                    text: Some(_),
                    file: None
                },
                ..
            }
        ));
        assert!(TestCli::try_parse_from(["checklist", "append", "PROJ-1"]).is_err());
        assert!(TestCli::try_parse_from([
            "checklist",
            "append",
            "PROJ-1",
            "--text",
            "item",
            "--file",
            "-"
        ])
        .is_err());
        assert!(TestCli::try_parse_from(["checklist", "update", "PROJ-1"]).is_err());
        assert!(
            TestCli::try_parse_from(["checklist", "update", "PROJ-1", "102", "--file", "-"])
                .is_err()
        );
        assert!(TestCli::try_parse_from([
            "checklist",
            "update",
            "PROJ-1",
            "--file",
            "-",
            "--label",
            "oops"
        ])
        .is_err());
        let cli = TestCli::try_parse_from([
            "checklist",
            "update",
            "PROJ-1",
            "102",
            "--mandatory",
            "false",
            "--status-id",
            "37",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            ChecklistCommand::Update {
                mandatory: Some(false),
                status_id: Some(37),
                ..
            }
        ));
        assert!(TestCli::try_parse_from(["checklist", "template", "list", "--global"]).is_err());
        assert!(
            TestCli::try_parse_from(["checklist", "template", "create", "--file", "-"]).is_ok()
        );
        assert!(TestCli::try_parse_from([
            "checklist",
            "replace",
            "PROJ-1",
            "--file",
            "-",
            "--force"
        ])
        .is_ok());
    }

    #[test]
    fn rendering_preserves_headings_lists_assignees_and_short_links() {
        let data = serde_json::from_str(include_str!(
            "../../../jira-core/tests/fixtures/smart-checklist.json"
        ))
        .unwrap();
        let out = render_checklists(&data, false);
        assert!(out.contains("# Release preparation [item 101]"));
        assert!(out.contains("- [TO DO / 1] Review jira cli @developer [item 102] (mandatory)"));
        assert!(out.contains("@developer (Developer)"));
        assert!(out.contains("  > * Review changes\n  > * Run checks"));
        assert!(!out.contains("https://"));
        assert!(!out.contains('\x1b'));
        assert!(render_checklists(&data, true)
            .contains("\x1b]8;;https://example.com/projects/jira\x1b\\jira cli\x1b]8;;\x1b\\"));
    }

    #[test]
    fn links_handle_unicode_parentheses_and_strip_server_escape_sequences() {
        assert_eq!(
            display_links(
                "[Café](https://example.com/page_(one)) and [two](https://example.com/two)",
                false
            ),
            "Café and two"
        );
        assert_eq!(
            display_links("[broken](https://example.com", false),
            "[broken](https://example.com"
        );
        assert_eq!(
            display_links("[file](file:///tmp/foo)", true),
            "[file](file:///tmp/foo)"
        );
        assert_eq!(
            terminal_text("safe\x1b]52;bad\x07\n* Item"),
            "safe]52;bad\n* Item"
        );
    }

    #[test]
    fn reads_utf8_and_reports_missing_files() {
        let path = std::env::temp_dir().join(format!("jirac-checklist-{}.txt", std::process::id()));
        std::fs::write(&path, "# Heading\n- Item").unwrap();
        assert_eq!(read_input(&path).unwrap(), "# Heading\n- Item");
        std::fs::remove_file(&path).unwrap();
        assert!(read_input(&path).is_err());
    }

    #[test]
    fn non_interactive_confirmation_requires_force() {
        super::super::interactive::init(true);
        assert!(confirm("Clear checklist?", false)
            .unwrap_err()
            .to_string()
            .contains("--force"));
        assert!(confirm("Clear checklist?", true).unwrap());
    }

    #[tokio::test]
    async fn disabled_feature_blocks_cli_before_reading_input_or_sending_requests() {
        let client = JiraClient::new(jira_core::config::JiraConfig::default());
        let command = ChecklistCommand::Append {
            target: ChecklistTarget {
                key: "PROJ-123".into(),
                checklist_id: None,
            },
            input: ChecklistText {
                text: None,
                file: Some(PathBuf::from("/missing-checklist-input.txt")),
            },
            json: false,
        };
        let error = handle(command, client).await.unwrap_err();
        assert!(error.to_string().contains("Smart Checklist is disabled"));
    }
}
