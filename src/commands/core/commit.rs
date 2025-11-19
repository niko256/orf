use crate::commands::core::index::idx_main::Index;
use crate::storage::objects::commit::Commit;
use crate::storage::objects::tree::{create_tree, store_tree};
use crate::storage::utils::Storable;
use crate::storage::utils::{HEAD_DIR, INDEX_FILE, OBJ_DIR, VOX_DIR};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Takes a commit message and optional author information
pub fn commit_command(message: &String, author: Option<String>) -> Result<()> {
    // Verify we're in a VOX repository
    if !PathBuf::from(&*VOX_DIR).exists() {
        return Err(anyhow::anyhow!("Not a vox repository (or any parent)"));
    }

    // Check if there are any staged changes to commit
    let index_path = PathBuf::from(&*INDEX_FILE);
    if !index_path.exists() {
        return Err(anyhow::anyhow!(
            "Nothing to commit (create/copy files and use 'vox add' to track)"
        ));
    }

    // Create a tree object from the current directory state
    let tree = create_tree(Path::new("."))?;
    let tree_hash = store_tree(&tree)?;

    // Get the hash of the current commit (if any) as parent
    let parent_commit = get_current_commit().context("Failed to get current commit")?;

    // Use provided author or default to unknown
    let author = author.unwrap_or_else(|| String::from("Unknown <unknown@example.com>"));

    // Create and save the new commit object
    let commit = Commit::new(tree_hash, parent_commit, author, message.to_string());
    let hash = commit.save(&PathBuf::from(&*OBJ_DIR))?;

    // Update the current branch to point to the new commit
    update_current_branch(&hash)?;

    let mut index = Index::new();
    index.read_from_file(&index_path)?;
    index.write_to_file(&*INDEX_FILE.as_ref())?;

    // Print commit confirmation (first 7 chars of hash + message)
    println!("[{}] {}", &hash[..7], commit.message);

    Ok(())
}

/// Retrieves the hash of the current commit from HEAD
/// Returns None if there's no commit yet
pub fn get_current_commit() -> Result<Option<String>> {
    let head_content = fs::read_to_string(&*HEAD_DIR).context("Failed to read HEAD file")?;

    if head_content.starts_with("ref: ") {
        // HEAD points to a branch reference
        let branch_ref = head_content.trim_start_matches("ref: ").trim();
        let ref_path = PathBuf::from(&*VOX_DIR).join(branch_ref);

        if ref_path.exists() {
            // Read and return the commit hash from the branch reference file
            let commit_hash = fs::read_to_string(&ref_path)
                .context("Failed to read branch reference")?
                .trim()
                .to_string();
            Ok(Some(commit_hash))
        } else {
            // Branch exists but has no commits yet
            Ok(None)
        }
    } else {
        // HEAD contains a direct commit hash (detached HEAD state)
        Ok(Some(head_content.trim().to_string()))
    }
}

/// Updates the current branch or HEAD to point to a new commit
pub fn update_current_branch(commit_hash: &str) -> Result<()> {
    let head_content = fs::read_to_string(&*HEAD_DIR).context("Failed to read HEAD file")?;

    if head_content.starts_with("ref: ") {
        // Update branch reference
        let branch_ref = head_content.trim_start_matches("ref: ").trim();
        let ref_path = PathBuf::from(&*VOX_DIR).join(branch_ref);

        // Ensure parent directories exist
        if let Some(parent) = ref_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write the new commit hash to the branch reference file
        fs::write(&ref_path, format!("{}\n", commit_hash))
            .context("Failed to update branch reference")?;
    } else {
        // Update HEAD directly in detached state
        fs::write(&*HEAD_DIR, format!("{}\n", commit_hash)).context("Failed to update HEAD")?;
    }

    Ok(())
}
