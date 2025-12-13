use crate::commands::core::index::idx_main::Index;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

////////////////////////////////////////////////////////////////////////////////

pub fn rm_command(paths: &[PathBuf], cached: bool, forced: bool) -> Result<()> {
    let index_path = Path::new(".vox/index");
    let mut index = Index::new();

    if index_path.exists() {
        index.read_from_file(index_path)?;
    } else {
        println!("Index is empty, nothing to remove");
        return Ok(());
    }

    let mut removed_count = 0;

    for path in paths {
        if path.is_dir() {
            for entry in WalkDir::new(path)
                .min_depth(1)
                .into_iter()
                .filter_entry(|e| e.file_type().is_file())
            {
                let entry = entry.with_context(|| format!("Failed to read directory entry"))?;
                let entry_path = entry.path().to_path_buf();
                removed_count += remove_single_file(&mut index, &entry_path, cached, forced)?;
            }
        } else {
            removed_count += remove_single_file(&mut index, path, cached, forced)?;
        }
    }

    if removed_count > 0 {
        index.write_to_file(index_path)?;
        println!("Removed {} entries from index", removed_count);
    } else {
        println!("No matching entries found to remove");
    }

    Ok(())
}

fn remove_single_file(index: &mut Index, path: &Path, cached: bool, forced: bool) -> Result<u32> {
    let relative_path = if path.starts_with("./") {
        path.strip_prefix("./").unwrap_or(path)
    } else {
        path
    };

    if index.get_entry(relative_path).is_none() {
        println!("Warning: '{}' not found in index", relative_path.display());
        return Ok(0);
    }

    if !cached && !forced && !relative_path.exists() {
        println!(
            "Warning: '{}' not found in working directory",
            relative_path.display()
        );
        return Ok(0);
    }

    if forced && relative_path.exists() {
        fs::remove_file(relative_path)
            .with_context(|| format!("Failed to remove file: {}", relative_path.display()))?;
        println!("Removed file: {}", relative_path.display());
    }

    index.remove_entry(relative_path);

    Ok(1)
}
