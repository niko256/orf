use crate::storage::utils::{HEAD_DIR, VOX_DIR};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/////////////////////////////

pub struct Branch {
    pub name: String,
    pub commit_hash: String,
}

impl Branch {
    /// Creates a new branch pointing to a specific commit
    pub fn new(name: &str, commit_hash: &str) -> Result<Self> {
        let branch_path = Self::get_branch_path(name);

        // Prevent duplicate branch names
        if branch_path.exists() {
            return Err(anyhow::anyhow!("Branch {} already exists", name));
        }

        // Create necessary directories
        if let Some(parent) = branch_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write commit hash to branch file
        fs::write(&branch_path, format!("\n{}", commit_hash))?;

        Ok(Self {
            name: name.to_string(),
            commit_hash: commit_hash.to_string(),
        })
    }

    fn get_branch_path(name: &str) -> PathBuf {
        PathBuf::from(&*VOX_DIR).join("refs/heads").join(name)
    }

    /// Deletes a branch if it exists and is not the current branch
    pub fn delete(&self) -> Result<()> {
        let branch_path = Self::get_branch_path(&self.name);

        if !branch_path.exists() {
            return Err(anyhow::anyhow!("Branch '{}' doesn't exist", self.name));
        }

        // Prevent deletion of current branch
        if let Some(current_branch) = Self::get_current_branch()? {
            if current_branch.name == self.name {
                return Err(anyhow::anyhow!("Cannot delete current branch!"));
            }
        }

        fs::remove_file(branch_path)?;
        Ok(())
    }

    /// Gets the currently checked out branch
    pub fn get_current_branch() -> Result<Option<Self>> {
        let head_content = fs::read_to_string(&*HEAD_DIR)?;

        // Parse HEAD file to find current branch
        if let Some(branch_name) = head_content.strip_prefix("ref: refs/heads/") {
            let branch_name = branch_name.trim();
            let branch_path = Self::get_branch_path(branch_name);

            if branch_path.exists() {
                let commit_hash = fs::read_to_string(&branch_path)?.trim().to_string();
                return Ok(Some(Self {
                    name: branch_name.to_string(),
                    commit_hash,
                }));
            }
        }
        Ok(None)
    }

    /// Lists all branches in the repository
    pub fn list() -> Result<Vec<Self>> {
        let mut branches = Vec::new();
        let refs_path = PathBuf::from(&*VOX_DIR).join("refs/heads");

        if !refs_path.exists() {
            return Ok(branches);
        }

        for entry in fs::read_dir(refs_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let commit_hash = fs::read_to_string(&path)?.trim().to_string();
                    branches.push(Self {
                        name: name.to_string(),
                        commit_hash,
                    });
                }
            }
        }
        // Sort branches alphabetically
        branches.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(branches)
    }
}
