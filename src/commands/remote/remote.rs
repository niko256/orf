use crate::commands::config::{
    conf_utils::get_local_config,
    config::{Config, PersistentConfig},
};
use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Subcommand)]
pub enum RemoteCommands {
    #[command(about = "Add a new remote repository")]
    Add {
        name: String,
        url: String,
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    #[command(about = "Remove a remote repository")]
    Remove { name: String },

    #[command(about = "Rename a remote repository")]
    Rename { old_name: String, new_name: String },

    #[command(about = "Show info about a remote repository")]
    Show { name: String },

    #[command(about = "List all remote repositories")]
    List,
}

pub fn is_valid_url(url: &str) -> bool {
    Url::parse(url).is_ok()
}

pub fn remote_command(command: &RemoteCommands) -> Result<()> {
    let config_path = get_local_config()?;

    let mut config = Config::read_from_file(&config_path)?;
    let mut config_changed = false;

    match command {
        RemoteCommands::Add { name, url, path } => {
            let workdir = match path {
                Some(p) => p.clone(),
                None => std::env::current_dir()
                    .with_context(|| format!("Failed to get current directory"))?,
            };
            config.add_remote(name, url, &workdir)?;
            config_changed = true;
            println!(
                "{} Added remote '{}' -> {}",
                "✓".green(),
                name.bold(),
                url.underline()
            );
        }
        RemoteCommands::Rename { old_name, new_name } => {
            config.rename_remote(old_name, new_name)?;
            config_changed = true;
            println!(
                "{} Renamed remote '{}' to '{}'",
                "✓".green(),
                old_name.bold(),
                new_name.bold()
            );
        }
        RemoteCommands::Remove { name } => {
            config.remove_remote(name)?;
            config_changed = true;
            println!("{} Removed remote '{}'", "✓".green(), name.bold());
        }
        RemoteCommands::List => {
            let current_config = Config::read_from_file(&config_path)?;
            if current_config.remotes().is_empty() {
                println!("{}", "No remotes found.".red());
            } else {
                println!("{}", "Remotes:".green());
                for remote in current_config.remotes() {
                    let url_str = remote.url().map(|u| u.as_str()).unwrap_or("N/A (Local)");
                    println!("  {:<15} {}", remote.name().cyan(), url_str.blue());
                }
            }
        }
        RemoteCommands::Show { name } => {
            let current_config = Config::read_from_file(&config_path)?;
            let remote = current_config.get_remote(name)?;

            println!("{} {}", "Remote:".bold(), remote.name().cyan());
            if let Some(url) = remote.url() {
                println!("  {:<10} {}", "URL:", url.as_str().blue());
            } else {
                println!("  {:<10} {}", "Type:", "Local".yellow());
            }
            println!("  {:<10} {}", "Workdir:", remote.workdir().display());
        }
    }

    if config_changed {
        config.write_to_file(&config_path)?;
    }

    Ok(())
}
