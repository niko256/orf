use anyhow::Context;
use std::path::Path;
use tokio::fs;
use tokio::io;

//////////////////////////////////////////////////////////////////////////////////////////////

/// Write to ref directory with given name and hash
pub async fn write_ref(refs_dir: &Path, ref_name: &str, commit_hash: &str) -> io::Result<()> {
    let ref_path = refs_dir.join(ref_name);
    if let Some(parent) = ref_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::write(&ref_path, format!("{commit_hash}\n")).await?;
    Ok(())
}

/// Read ref directory
pub async fn read_ref(refs_dir: &Path, ref_name: &str) -> anyhow::Result<String> {
    let ref_path = refs_dir.join(ref_name);
    let data = fs::read(&ref_path)
        .await
        .with_context(|| format!("Failed to read ref: {:?}", ref_path))?;

    let content = String::from_utf8(data)
        .with_context(|| format!("Ref file contains invalid UTF-8: {:?}", ref_path))?;
    Ok(content.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_w_r_refs() {
        let tmp_dir = tempdir().unwrap();
        let refs_dir = tmp_dir.path().join("refs").join("heads");

        let ref_name = "main";
        let commit_hash = "ei4398fiirufiue939r8hfdojfjer404849893fjef";
        write_ref(&refs_dir, ref_name, commit_hash).await.unwrap();
        let read = read_ref(&refs_dir, ref_name).await.unwrap();

        assert_eq!(read, commit_hash);
    }
}
