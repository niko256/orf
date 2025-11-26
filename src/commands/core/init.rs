use crate::{
    commands::core::index::idx_main::Index,
    storage::utils::{HEAD_DIR, INDEX_FILE, OBJ_DIR, REFS_DIR, VOX_DIR},
};
use anyhow::{Context, Result};
use std::path::Path;

///////////////////////////////////////////////////////////////////////////////

pub fn init_command() -> Result<()> {
    std::fs::create_dir_all(&*VOX_DIR).context("Failed to create .vox directory")?;
    std::fs::create_dir_all(&*OBJ_DIR).context("Failed to create .vox/objects directory")?;
    std::fs::create_dir_all(&*REFS_DIR).context("Failed to create .vox/refs directory")?;
    std::fs::write(&*HEAD_DIR, "ref: refs/heads/main\n")
        .context("Failed to write to .vox/HEAD file")?;

    let index = Index::new();
    index
        .write_to_file(Path::new(&*INDEX_FILE))
        .context("Failed to create index file")?;

    println!("Initialized vox directory");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test_repo");

        std::fs::create_dir_all(&repo_path).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo_path).unwrap();

        init_command().unwrap();

        std::env::set_current_dir(original_dir).unwrap();

        assert!(repo_path.join(".vox").exists());
        assert!(repo_path.join(".vox/objects").exists());
        assert!(repo_path.join(".vox/refs").exists());
        assert!(repo_path.join(".vox/HEAD").exists());
        assert!(repo_path.join(".vox/index").exists());
    }
}
