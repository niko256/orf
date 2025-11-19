use crate::commands::core::commit::get_current_commit;
use crate::commands::core::index::idx_main::Index;
use anyhow::{Context, Result};
use std::collections::hash_set::HashSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Entry point for the `status` command.
/// Displays the current status of the working directory, including:
/// - Added files
/// - Modified files
/// - Deleted files
/// - Untracked files
pub fn status_command() -> Result<()> {
    let (added, modified, deleted, untracked) = get_status(Path::new("."))?;

    // Retrieve the current commit hash
    let current_commit = get_current_commit()?;

    print_status(&added, &modified, &deleted, &untracked, current_commit);
    Ok(())
}

/// Represents the status of files in the working directory.
#[derive(Default)]
struct FileStatus {
    added: Vec<PathBuf>,     // Files added to the index
    modified: Vec<PathBuf>,  // Files modified after being staged
    deleted: Vec<PathBuf>,   // Files deleted from the working directory
    untracked: Vec<PathBuf>, // Files not tracked by the index
}

/// Computes the status of the working directory compared to the index.
/// # Arguments
/// - `repo_path`: The path to the repository root.
pub fn get_status(
    repo_path: &Path,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>)> {
    let mut index = Index::new();
    let index_path = repo_path.join(".vox/index");

    // Load the index if it exists
    if index_path.exists() {
        index.read_from_file(&index_path)?;
    }

    let mut status = FileStatus::default();

    let mut processed_files = HashSet::new();

    // Iterate over files in the index
    for (path, index_entry) in index.get_entries().iter() {
        processed_files.insert(path.clone());
        let full_path = repo_path.join(path);

        // Check if the file exists in the working directory
        if !full_path.exists() {
            // File is deleted
            status.deleted.push(path.clone());
        } else {
            // Compare metadata to detect modifications
            let metadata = fs::metadata(&full_path)?;
            if metadata.mtime() as u64 != index_entry.mtime
                || metadata.size() as u32 != index_entry.size
            {
                // File is modified
                status.modified.push(path.clone());
            } else {
                // File is added (unchanged)
                status.added.push(path.clone());
            }
        }
    }

    // Walk through the working directory to find untracked files
    for entry in WalkDir::new(repo_path)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| {
            // Ignore specific directories
            !e.path().starts_with(repo_path.join(".vox"))
                && !e.path().starts_with(repo_path.join(".git"))
                && !e.path().starts_with(repo_path.join("target"))
        })
    {
        let entry = entry.context("Failed to read directory entry")?;
        if !entry.file_type().is_file() {
            continue; // Skip non-file entries
        }

        // Get the relative path of the file
        let relative_path = entry.path().strip_prefix(repo_path)?.to_path_buf();

        // Check if the file is already processed
        if !processed_files.contains(&relative_path) {
            if index.get_entries().contains_key(&relative_path) {
                // File is in the index; check if it's modified
                let metadata = fs::metadata(entry.path())?;
                let index_entry = index.get_entries().get(&relative_path).unwrap();
                if metadata.mtime() as u64 != index_entry.mtime
                    || metadata.size() as u32 != index_entry.size
                {
                    // File is modified
                    status.modified.push(relative_path);
                }
            } else {
                // File is untracked
                status.untracked.push(relative_path);
            }
        }
    }

    // Return the computed status
    Ok((
        status.added,
        status.modified,
        status.deleted,
        status.untracked,
    ))
}

/// Prints the status of the working directory to the console.
///
fn print_status(
    added: &[PathBuf],
    modified: &[PathBuf],
    deleted: &[PathBuf],
    untracked: &[PathBuf],
    current_commit: Option<String>,
) {
    // Get the current branch name
    let branch_name = match get_current_branch() {
        Ok(name) => name,
        Err(_) => "unknown".to_string(),
    };

    // Print branch and commit information
    println!("On branch {}", branch_name);
    if let Some(commit) = current_commit {
        println!("Current commit [{}]", &commit[..7]); // Display the first 7 characters of the commit hash
    }

    // Check if the working tree is clean
    if added.is_empty() && modified.is_empty() && deleted.is_empty() && untracked.is_empty() {
        println!("✓ Working tree clean");
        return;
    }

    // Print added files
    if !added.is_empty() {
        println!("Changes to be committed:");
        println!("  (use \"vox reset HEAD <file>...\" to unstage)\n");
        for path in added {
            println!("\t\x1b[32mnew file:   {}\x1b[0m", path.display()); // Green color for added files
        }
        println!();
    }

    // Print modified and deleted files
    if !modified.is_empty() || !deleted.is_empty() {
        println!("Changes not added for commit:");
        println!("  (use \"vox add <file>...\" to update what will be committed)");
        println!("  (use \"vox restore <file>...\" to discard changes)\n");

        for path in modified {
            println!("\t\x1b[31mmodified:   {}\x1b[0m", path.display());
        }
        for path in deleted {
            println!("\t\x1b[31mdeleted:    {}\x1b[0m", path.display());
        }
        println!();
    }

    // Print untracked files
    if !untracked.is_empty() {
        println!("Untracked files:");
        println!("  (use \"vox add <file>...\" to include in what will be committed)\n");
        for path in untracked {
            println!("\t\x1b[31m{}\x1b[0m", path.display()); // Red color for untracked files
        }
        println!();
    }

    // Print a summary message
    if !modified.is_empty() || !untracked.is_empty() {
        println!("no changes added to commit (use \"vox add\" and/or \"vox commit -a\")");
    }
}

/// Retrieves the name of the current branch.
///
fn get_current_branch() -> Result<String> {
    let head_content = fs::read_to_string(".vox/HEAD").context("Failed to read HEAD file")?;

    // Parse the branch name from the HEAD file (e.g., "ref: refs/heads/branch_name")
    let branch = head_content
        .strip_prefix("ref: refs/heads/")
        .and_then(|s| s.strip_suffix('\n'))
        .context("Invalid HEAD file format")?;

    Ok(branch.to_string())
}
