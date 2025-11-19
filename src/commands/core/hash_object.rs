use crate::{storage::objects::blob::Blob, storage::utils::OBJ_DIR};
use anyhow::Result;
use clap::Parser;
use std::fs;

#[derive(Parser, Debug)]
pub struct HashObjectArgs {
    pub file_path: String,
}

pub fn hash_object_command(args: HashObjectArgs) -> Result<()> {
    fs::create_dir_all(&*OBJ_DIR)?;
    let object_hash = Blob::blob_hash(&args.file_path)?;
    println!("{}", object_hash);
    Ok(())
}

#[cfg(test)]
mod tests {

    use assert_cmd::Command;
    use predicates::prelude::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_hash_object() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        std::env::set_current_dir(dir.path())?;

        let file_path = dir.path().join("test_file.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "test content")?;

        let mut cmd = Command::cargo_bin("vox")?;
        cmd.arg("hash-object").arg(file_path.to_str().unwrap());

        cmd.assert()
            .success()
            .stdout(predicate::str::is_match(r"[a-f0-9]{40}").unwrap());

        Ok(())
    }

    #[test]
    fn test_help_command() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("vox")?;
        cmd.arg("help");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Usage: vox <COMMAND>"));

        Ok(())
    }

    #[test]
    fn test_init_command() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let mut cmd = Command::cargo_bin("vox")?;
        cmd.arg("init").current_dir(dir.path());

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Initialized vox directory"));

        assert!(dir.path().join(".vox").exists());
        assert!(dir.path().join(".vox/objects").exists());
        assert!(dir.path().join(".vox/refs").exists());
        assert!(dir.path().join(".vox/HEAD").exists());

        Ok(())
    }

    #[test]
    fn test_integration() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let mut cmd = Command::cargo_bin("vox")?;
        cmd.arg("init").current_dir(dir.path());
        cmd.assert().success();

        let file_path = dir.path().join("test_file.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "test content")?;

        let mut cmd = Command::cargo_bin("vox")?;
        cmd.arg("hash-object").arg(file_path.to_str().unwrap());
        let output = cmd.output()?;
        let hash = String::from_utf8(output.stdout)?.trim().to_string();

        let mut cmd = Command::cargo_bin("vox")?;
        cmd.arg("cat-file").arg("-p").arg(hash);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("test content"));

        Ok(())
    }
}
