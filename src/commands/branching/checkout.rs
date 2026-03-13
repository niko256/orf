use crate::commands::core::status::get_status;
use crate::storage::objects::branch::Branch;
use crate::storage::objects::commit::Commit;
use crate::storage::objects::tree::read_tree;
use crate::storage::utils::Loadable;
use crate::storage::utils::{HEAD_DIR, OBJ_DIR, OBJ_TYPE_BLOB, OBJ_TYPE_TREE};
use anyhow::{Context, Result};
use colored::*;
use flate2::bufread::ZlibDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use sha1::*;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Main checkout command that switches between branches or commits
/// Parameters:
/// - target: branch name or commit hash to checkout
/// - force: whether to force checkout even with uncommitted changes
pub fn checkout_command(target: &str, force: bool, workdir: Option<&Path>) -> Result<()> {
    let _workdir = workdir.unwrap_or_else(|| Path::new("."));

    // Check for uncommitted changes unless force flag is set
    if !force {
        let (_added, modified, deleted, untracked) = get_status(Path::new("."))?;
        if !modified.is_empty() || !deleted.is_empty() || !untracked.is_empty() {
            return Err(anyhow::anyhow!(
                "You have uncommitted changes. Commit or stash them first (or use --force)"
                    .red()
                    .to_string()
            ));
        }
    }

    // Determine if target is a commit hash (40 chars) or branch name
    let commit_hash = if target.len() == 40 {
        target.to_string()
    } else {
        // Look up branch and get its commit hash
        match Branch::list()?.iter().find(|b| b.name == target) {
            Some(branch) => branch.commit_hash.clone(),
            None => return Err(anyhow::anyhow!("Branch or commit '{}' not found", target)),
        }
    };

    // Load the target commit
    let commit = Commit::load(&commit_hash, &PathBuf::from(&*OBJ_DIR))?;

    // Clean working directory before checkout
    clean_working_directory(Path::new("."));

    // Restore files from commit's tree
    restore_tree(&commit.tree, Path::new("."))?;

    // Update HEAD to point to new commit/branch
    if target.len() == 40 {
        fs::write(&*HEAD_DIR, commit_hash)?; // Direct commit reference
    } else {
        fs::write(&*HEAD_DIR, format!("ref: refs/heads/{}\n", target))?; // Branch reference
    }

    println!("Succesfully checked out {}", target);
    Ok(())
}

/// Cleans the working directory by removing all files and directories
/// except hidden files and special directories (.orf, .git, target)
fn clean_working_directory(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files/directories
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("."))
            .unwrap_or(false)
        {
            continue;
        }

        if path.is_dir() {
            // Skip special directories
            if path.starts_with(".orf") || path.starts_with(".git") || path.starts_with("target") {
                continue;
            }
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// Recursively restores a tree object to the filesystem
/// Shows progress bar for visual feedback
fn restore_tree(tree_hash: &str, base_path: &Path) -> Result<()> {
    let tree = read_tree(tree_hash, &*OBJ_DIR)?;
    // Setup progress bar
    let pb = ProgressBar::new(tree.entries.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {percent}% • {pos}/{len} files {msg}")
            .unwrap()
            .progress_chars("▰▰▱"),
    );

    // Process each entry in the tree
    for entry in tree.entries {
        let path = base_path.join(&entry.name);
        pb.set_prefix(format!("Processing: {}", entry.name));

        match entry.object_type.as_str() {
            OBJ_TYPE_TREE => {
                fs::create_dir_all(&path)?;
                let _ = restore_tree(&entry.object_hash, &path);
            }
            OBJ_TYPE_BLOB => {
                restore_blob(&entry.object_hash, &path)?;
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unknown object type: {}",
                    entry.object_type
                ));
            }
        }
        pb.inc(1);
    }
    pb.finish_with_message("Files restored Succesfully!");
    Ok(())
}

/// Restores a blob (file) object to the filesystem
/// Only updates if file doesn't exist or content has changed
fn restore_blob(hash: &str, path: &Path) -> Result<()> {
    if !should_update_file(path, hash) {
        return Ok(());
    }

    // Construct path to blob object
    let object_path = PathBuf::from(&*OBJ_DIR).join(&hash[..2]).join(&hash[2..]);

    // Read and decompress blob data
    let compressed_data =
        fs::read(&object_path).with_context(|| format!("Failed to read object {}", hash))?;

    let mut decoder = ZlibDecoder::new(&compressed_data[..]);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;

    // Find content after header
    let content_start = decompressed_data
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid blob format"))?;

    let content = &decompressed_data[content_start + 1..];

    // Ensure parent directory exists and write file
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Determines if a file needs to be updated by comparing its hash
/// with the expected hash from the repository
fn should_update_file(path: &Path, expected_hash: &str) -> bool {
    if !path.exists() {
        return true;
    }

    if let Ok(current_content) = fs::read(path) {
        let mut hasher = Sha1::new();
        hasher.update(&current_content);
        let current_hash = format!("{:x}", hasher.finalize());
        current_hash != expected_hash
    } else {
        true
    }
}
