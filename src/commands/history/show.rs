use crate::commands::core::commit::get_current_commit;
use crate::storage::objects::commit::Commit;
use crate::storage::objects::tree::read_tree;
use crate::storage::utils::{Loadable, OBJ_DIR, OBJ_TYPE_BLOB, OBJ_TYPE_TREE};
use anyhow::Result;
use chrono::{DateTime, Local};
use colored::*;
use std::path::PathBuf;

/// Entry point for the `show` command.
/// Displays detailed information about a specific commit, including:
/// - Commit hash
/// - Author
/// - Date
/// - Commit message
/// - Changes (tree structure)
/// - Parent commit (if available)
///
/// # Arguments
/// - `commit_ref`: The commit reference (e.g., "HEAD" or a commit hash).
///
pub fn show_command(commit_ref: &str) -> Result<()> {
    // Resolve the commit hash
    let commit_hash = if commit_ref == "HEAD" {
        // If "HEAD" is provided, get the current commit hash
        get_current_commit()?.ok_or_else(|| anyhow::anyhow!("No commits yet!"))?
    } else {
        // Otherwise, use the provided commit reference
        commit_ref.to_string()
    };

    // Load the commit object
    let commit = Commit::load(&commit_hash, &PathBuf::from(&*OBJ_DIR))?;

    // Print the commit details
    print_commit_details(&commit_hash, &commit)?;

    Ok(())
}

/// Prints detailed information about a commit.
///
/// # Arguments
/// - `hash`: The commit hash.
/// - `commit`: The commit object.
///
fn print_commit_details(hash: &str, commit: &Commit) -> Result<()> {
    let local_date: DateTime<Local> = commit.timestamp.with_timezone(&Local);
    let formatted_date = local_date.format("%Y-%m-%d %H:%M:%S %z");

    // Print commit metadata
    println!("{}", "=".repeat(70).blue());
    println!("{} {}", "Commit:".yellow(), hash.bright_purple()); // Commit hash
    println!("{} {}", "Author:".cyan(), commit.author); // Author
    println!("{} {}", "Date:".cyan(), formatted_date); // Commit date
    println!("\n{}", commit.message.bright_white()); // Commit message
    println!("{}", "=".repeat(70).blue());

    // Print changes
    println!("\n{}", "Changes:".green().bold());
    print_tree_info(&commit.tree, "", true)?;

    // Print parent commit information (if available)
    if let Some(parent) = &commit.parent {
        println!("\n{}", "Parent commit:".yellow());

        // Load the parent commit
        let parent_commit = Commit::load(parent, &PathBuf::from(&*OBJ_DIR))?;
        println!(
            "  {} {}",
            parent[..8].bright_purple(), // Shortened parent commit hash
            parent_commit.message.split('\n').next().unwrap_or("") // First line of the parent commit message
        );
    }

    Ok(())
}

/// Recursively prints the tree structure of a commit.
///
/// # Arguments
/// - `tree_hash`: The hash of the tree to print.
/// - `prefix`: The prefix for indentation (used for recursive calls).
/// - `_is_last`: Whether the current tree entry is the last in its parent tree.
///
fn print_tree_info(tree_hash: &str, prefix: &str, _is_last: bool) -> Result<()> {
    let tree = read_tree(tree_hash, &*OBJ_DIR)?;
    let entries = tree.entries;

    // Iterate over the entries in the tree
    for (idx, entry) in entries.iter().enumerate() {
        let is_last_entry = idx == entries.len() - 1; // Check if this is the last entry
        let branch = if is_last_entry {
            "└── "
        } else {
            "├── "
        };
        let next_prefix = if is_last_entry { "    " } else { "│   " }; // Indentation for nested entries

        // Colorize the entry name based on its type
        let display = match entry.object_type.as_str() {
            OBJ_TYPE_TREE => entry.name.blue(),   // Directories are blue
            OBJ_TYPE_BLOB => entry.name.normal(), // Files are normal
            _ => entry.name.red(),                // Unknown types are red
        };

        // Print the entry
        println!(
            "{}{}{}    {}",
            prefix,                                            // Indentation
            branch.purple(),                                   // Branch symbol
            display,                                           // Entry name
            format!("[{}]", &entry.object_hash[..8]).dimmed()  // Shortened object hash
        );

        // If the entry is a tree, recursively print its contents
        if entry.object_type == "tree" {
            print_tree_info(
                &entry.object_hash,
                &format!("{}{}", prefix, next_prefix), // Update the prefix for indentation
                is_last_entry,
            )?;
        }
    }
    Ok(())
}
