use crate::commands::core::index::idx_main::{Index, IndexEntry};
use crate::storage::objects::blob::Blob;
use anyhow::{Context, Result};
use std::{
    env,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Represents the add command functionality for staging files
pub struct AddCommand {
    repo_root: PathBuf,   // Root directory of the repository
    index: Index,         // Staging area index
    current_dir: PathBuf, // Current working directory
}

impl AddCommand {
    /// Finds repository root and initializes/loads the index
    pub fn new() -> Result<Self> {
        let repo_root = Self::find_repository_root()?;
        let current_dir = env::current_dir()?;
        let index = Self::load_or_create_index(&repo_root)?;

        Ok(Self {
            repo_root,
            index,
            current_dir,
        })
    }

    /// Executes the add command for given paths
    /// Handles relative path conversion and adds files to index
    pub fn execute(mut self, paths: &[PathBuf]) -> Result<()> {
        // Convert current directory to repository-relative path
        let relative_base = self
            .current_dir
            .strip_prefix(&self.repo_root)?
            .to_path_buf();

        // Process each path provided
        for path in paths {
            self.add_path(path, &relative_base)?;
        }

        self.save_index()
    }

    /// Recursively finds the repository root by looking for (.vox) directory
    fn find_repository_root() -> Result<PathBuf> {
        let mut current = env::current_dir()?;
        loop {
            if current.join(".vox").is_dir() {
                return Ok(current);
            }
            if !current.pop() {
                return Err(anyhow::anyhow!("Not a vox repository (or any parent)"));
            }
        }
    }

    /// Loads existing index or creates new one if it doesn't exist
    fn load_or_create_index(repo_root: &Path) -> Result<Index> {
        let mut index = Index::new();
        let index_path = repo_root.join(".vox/index");

        if index_path.exists() {
            index.read_from_file(&index_path)?;
        }

        Ok(index)
    }

    /// Adds a single path to the index
    /// Handles both files and directories recursively
    fn add_path(&mut self, path: &Path, relative_base: &Path) -> Result<()> {
        let absolute_path = self.current_dir.join(path);
        let repo_root = self.repo_root.clone();

        if absolute_path.is_file() {
            // Handle single file
            let relative_path = if let Ok(rel) = path.strip_prefix(&self.repo_root) {
                rel.to_path_buf()
            } else {
                relative_base.join(path)
            };
            self.create_index_entry(&absolute_path, &relative_path)?;
        } else if absolute_path.is_dir() {
            // Handle directory recursively
            // Filter out VOX directories and build artifacts
            for entry in WalkDir::new(&absolute_path)
                .min_depth(1)
                .into_iter()
                .filter_entry(move |e| {
                    !e.path().starts_with(repo_root.join(".vox"))
                        && !e.path().starts_with(repo_root.join(".git"))
                        && !e.path().starts_with(repo_root.join("target"))
                        && !e.path().starts_with(repo_root.join("build"))
                })
            {
                let entry = entry.context("Failed to read directory entry")?;
                if !entry.file_type().is_file() {
                    continue;
                }

                // Convert to repository-relative path
                let relative_path = entry.path().strip_prefix(&self.current_dir)?.to_path_buf();
                let relative_path = relative_base.join(&relative_path);

                self.create_index_entry(entry.path(), &relative_path)?;
            }
        } else {
            return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
        }

        Ok(())
    }

    /// Creates an index entry for a file
    /// Generates blob hash and updates index
    fn create_index_entry(&mut self, abs_path: &Path, rel_path: &Path) -> Result<()> {
        // Create blob object from file content
        let blob_hash = Blob::blob_hash(
            abs_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid path"))?,
        )?;

        // Convert hex hash to bytes
        let hash_bytes = hex::decode(blob_hash.as_str())
            .with_context(|| format!("Failed to decode blob hash: {}", blob_hash))?;

        // Create and update index entry
        let mut entry = IndexEntry::new(abs_path)?;
        entry.path = rel_path.to_path_buf();
        entry.hash.copy_from_slice(&hash_bytes);

        self.index.add_entry(entry);
        Ok(())
    }

    /// Saves the current index state to disk
    fn save_index(&self) -> Result<()> {
        let index_path = self.repo_root.join(".vox/index");
        self.index.write_to_file(&index_path)
    }
}

pub fn add_command(paths: &[PathBuf]) -> Result<()> {
    AddCommand::new()?.execute(paths)
}
