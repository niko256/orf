use crate::{
    commands::core::index::idx_main::Index,
    storage::utils::{HEAD_DIR, INDEX_FILE, OBJ_DIR, REFS_DIR, VOX_DIR},
};
use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

pub async fn init_command() -> Result<()> {
    fs::create_dir_all(&*VOX_DIR)
        .await
        .context("Failed to create .vox directory")?;
    fs::create_dir_all(&*OBJ_DIR)
        .await
        .context("Failed to create .vox/objects directory")?;
    fs::create_dir_all(&*REFS_DIR)
        .await
        .context("Failed to create .vox/refs directory")?;
    fs::write(&*HEAD_DIR, "ref: refs/heads/main\n")
        .await
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
    use tokio::runtime::Runtime;

    #[test]
    fn test_async_init() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let repo_path = temp_dir.path().join("test_repo");

            fs::create_dir_all(&repo_path).await.unwrap();

            let original_dir = std::env::current_dir().unwrap();
            std::env::set_current_dir(&repo_path).unwrap();

            init_command().await.unwrap();

            std::env::set_current_dir(original_dir).unwrap();

            assert!(repo_path.join(".vox").exists());
            assert!(repo_path.join(".vox/objects").exists());
            assert!(repo_path.join(".vox/refs").exists());
            assert!(repo_path.join(".vox/HEAD").exists());
            assert!(repo_path.join(".vox/index").exists());
        });
    }
}
