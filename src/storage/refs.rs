use anyhow::Context;
use std::fs;
use std::path::Path;
use tokio::io;

//////////////////////////////////////////////////////////////////////////////////////////////

/// Write to ref directory with given name and hash
pub fn write_ref(refs_dir: &Path, ref_name: &str, commit_hash: &str) -> io::Result<()> {
    let ref_path = refs_dir.join(ref_name);
    if let Some(parent) = ref_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&ref_path, format!("{commit_hash}\n"))?;
    Ok(())
}

/// Read ref directory
pub fn read_ref(refs_dir: &Path, ref_name: &str) -> anyhow::Result<String> {
    let ref_path = refs_dir.join(ref_name);
    let data =
        fs::read(&ref_path).with_context(|| format!("Failed to read ref: {:?}", ref_path))?;

    let content = String::from_utf8(data)
        .with_context(|| format!("Ref file contains invalid UTF-8: {:?}", ref_path))?;
    Ok(content.trim().to_string())
}
