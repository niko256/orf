use crate::storage::utils::{Loadable, OBJ_DIR};
use crate::{commands::core::commit::get_current_commit, storage::objects::commit::Commit};
use anyhow::Result;
use chrono::{DateTime, Local};
use colored::*;
use std::path::PathBuf;

/// Displays the commit history, starting from the current commit (HEAD).
///
/// # Arguments
/// - `count`: The maximum number of commits to display.
///
pub fn log_command(count: usize) -> Result<()> {
    let mut current_commit_hash = get_current_commit()?;

    if current_commit_hash.is_none() {
        println!("{}", "No commits yet.".yellow());
        return Ok(());
    }

    println!("{}", "Commit History".bold().blue());
    println!("{}", "=".repeat(50).blue());

    // Track the number of commits shown
    let mut commits_shown = 0;

    // Traverse the commit history
    while let Some(commit_hash) = current_commit_hash {
        // Stop if the maximum number of commits has been shown
        if commits_shown >= count {
            break;
        }

        // Load the commit object
        let commit = Commit::load(&commit_hash, &PathBuf::from(&*OBJ_DIR))?;

        // Print the commit details
        print_commit(&commit_hash, &commit, commits_shown == 0);

        // Move to the parent commit
        current_commit_hash = commit.parent;
        commits_shown += 1;
    }

    // If there are more commits than the specified count, indicate that
    if commits_shown >= count {
        println!(
            "\n{}",
            format!("... and {} more commits", commits_shown).dimmed()
        );
    }

    Ok(())
}

/// Prints detailed information about a single commit.
///
fn print_commit(hash: &str, commit: &Commit, is_latest: bool) {
    let local_date: DateTime<Local> = commit.timestamp.with_timezone(&Local);
    let formatted_date = local_date.format("%Y-%m-%d %H:%M:%S %z");

    // Print commit metadata
    println!("{}", "┌".yellow()); // Top border
    if is_latest {
        println!("{}  {}", "│".yellow(), "HEAD -> main".green());
    }
    println!(
        "{}  {} {}",
        "│".yellow(),
        "commit".yellow(),
        hash.bright_yellow()
    );
    println!("{}  {} {}", "│".yellow(), "Author:".cyan(), commit.author);
    println!("{}  {} {}", "│".yellow(), "Date:".cyan(), formatted_date);
    println!("{}", "│".yellow());

    // Print the commit message (line by line)
    for line in commit.message.lines() {
        println!("{}      {}", "│".yellow(), line);
    }

    println!("{}\n", "└".yellow());
}
