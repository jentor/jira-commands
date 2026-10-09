use anyhow::{Context, Result};
use clap::Subcommand;
use jira_core::config::{config_file_path, JiraConfig, JiraProfilesFile};

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Set a configuration value for the active profile
    ///
    /// Supports `default_issue_limit` (0 means fetch all) and
    /// `smart_checklist_enabled` (true/false, disabled by default).
    Set {
        /// Configuration key (e.g. default_issue_limit)
        key: String,
        /// Configuration value (e.g. 100)
        value: String,
    },
    /// Remove a configuration value for the active profile,
    /// restoring its built-in default
    Unset {
        /// Configuration key (e.g. default_issue_limit)
        key: String,
    },
    /// Show the active profile's effective configuration
    Show,
}

pub async fn handle(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Set { key, value } => set_value(&key, &value),
        ConfigCommand::Unset { key } => unset_value(&key),
        ConfigCommand::Show => show(),
    }
}

fn set_value(key: &str, value: &str) -> Result<()> {
    match key {
        "smart_checklist_enabled" => {
            let enabled = value
                .trim()
                .parse::<bool>()
                .context("`smart_checklist_enabled` must be true or false")?;
            let mut config = JiraConfig::load()?;
            config.smart_checklist_enabled = enabled;
            config.save().context("Failed to save config")?;
            println!("✓ smart_checklist_enabled = {enabled}");
            println!("  Saved to {}", config_file_path().display());
        }
        "default_issue_limit" => {
            let parsed = value.trim().parse::<u32>().map_err(|_| {
                anyhow::anyhow!("`default_issue_limit` must be a non-negative integer")
            })?;
            let mut config = JiraConfig::load().unwrap_or_default();
            config.default_issue_limit = if parsed == 0 { None } else { Some(parsed) };
            config.save().context("Failed to save config")?;
            match config.default_issue_limit {
                Some(n) => println!("✓ default_issue_limit = {n}"),
                None => println!("✓ default_issue_limit cleared (will fetch all issues)"),
            }
            println!("  Saved to {}", config_file_path().display());
        }
        other => {
            anyhow::bail!("unsupported config key `{other}`. Supported: default_issue_limit, smart_checklist_enabled")
        }
    }
    Ok(())
}

fn unset_value(key: &str) -> Result<()> {
    match key {
        "smart_checklist_enabled" => {
            let mut config = JiraConfig::load()?;
            config.smart_checklist_enabled = false;
            config.save().context("Failed to save config")?;
            println!("✓ smart_checklist_enabled removed (disabled)");
        }
        "default_issue_limit" => {
            let mut config = JiraConfig::load().unwrap_or_default();
            config.default_issue_limit = None;
            config.save().context("Failed to save config")?;
            println!("✓ default_issue_limit removed (will fetch all issues)");
        }
        other => anyhow::bail!("unsupported config key `{other}`. Supported: default_issue_limit, smart_checklist_enabled"),
    }
    Ok(())
}

fn show() -> Result<()> {
    let config = JiraConfig::load().unwrap_or_default();
    let profile = config
        .profile_name
        .clone()
        .unwrap_or_else(|| "default".to_string());
    println!("Profile:        {profile}");
    println!(
        "Base URL:       {}",
        if config.base_url.is_empty() {
            "(not set)"
        } else {
            &config.base_url
        }
    );
    println!(
        "Project:        {}",
        config.project.clone().unwrap_or_else(|| "(not set)".into())
    );
    println!(
        "default_issue_limit: {}",
        config
            .default_issue_limit
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unset (fetch all)".into())
    );
    println!(
        "smart_checklist_enabled: {}",
        config.smart_checklist_enabled
    );

    let _ = JiraProfilesFile::load(); // validate load path
    Ok(())
}
