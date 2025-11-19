use super::tree::{Tree, read_tree};
use crate::storage::objects::change::ChangeSet;
use crate::storage::utils::{Loadable, OBJ_DIR, OBJ_TYPE_COMMIT, Storable, VoxObject};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// Represents a commit
///
/// A commit records a snapshot of the repository's state at a point in time,
/// including references to the root tree, parent commit(s), author information,
/// and commit message.
#[derive(PartialEq, Eq, Hash)]
pub struct Commit {
    /// Hash of the root tree object for this commit
    pub tree: String,
    /// Optional hash of the parent commit
    pub parent: Option<String>,
    /// Author of the commit (identifier)
    pub author: String,
    /// Timestamp when the commit was created
    pub timestamp: DateTime<Utc>,
    /// Commit message describing the changes
    pub message: String,
}

impl VoxObject for Commit {
    /// Returns the object type ("commit")
    fn object_type(&self) -> &str {
        OBJ_TYPE_COMMIT
    }

    /// Serializes the commit to bytes
    ///
    /// Format includes:
    /// - tree hash
    /// - parent hash (if exists)
    /// - author and timestamp
    /// - commit message
    ///
    fn serialize(&self) -> Result<Vec<u8>> {
        let mut content = Vec::new();

        content.extend(format!("tree {}\n", self.tree).as_bytes());

        if let Some(parent) = &self.parent {
            content.extend(format!("parent {}\n", parent).as_bytes());
        }

        let timestamp = self.timestamp.timestamp().to_string();
        content.extend(format!("author {} {}\n", self.author, timestamp).as_bytes());
        content.extend(b"\n");

        content.extend(self.message.as_bytes());
        content.extend(b"\n");

        Ok(content)
    }

    /// Computes the SHA-1 hash of the serialized commit
    fn hash(&self) -> Result<String> {
        let content = self.serialize()?;
        let mut hasher = Sha1::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Returns the storage path for this commit in the objects directory
    ///
    /// The path follows Git's convention: `objects/xx/yyyy...`
    /// where xx is the first two hex digits of the hash and yyyy... is the rest
    ///
    fn object_path(&self) -> Result<String> {
        let hash = self.hash()?;
        Ok(format!(
            "{}/{}/{}",
            OBJ_DIR.display(),
            &hash[..2],
            &hash[2..]
        ))
    }
}

impl Storable for Commit {
    /// Saves the commit object to the objects directory
    fn save(&self, objects_dir: &Path) -> Result<String> {
        let hash = self.hash()?;
        let content = self.serialize()?;

        let header = format!("commit {}\0", content.len());
        let full_content = [header.as_bytes(), &content].concat();

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&full_content)?;
        let compressed_data = encoder.finish()?;

        let dir_path = objects_dir.join(&hash[..2]);
        fs::create_dir_all(&dir_path)?;
        let object_path = dir_path.join(&hash[2..]);
        fs::write(&object_path, compressed_data)?;

        Ok(hash)
    }
}

impl Loadable for Commit {
    /// Loads a commit object from the objects directory
    fn load(hash: &str, objects_dir: &Path) -> Result<Self> {
        let dir_path = objects_dir.join(&hash[..2]);
        let object_path = dir_path.join(&hash[2..]);

        let compressed_data = fs::read(&object_path)?;
        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;

        let null_pos = decompressed_data
            .iter()
            .position(|&b| b == 0)
            .context("Invalid format: no null byte found")?;

        let header = std::str::from_utf8(&decompressed_data[..null_pos])?;
        if !header.starts_with("commit ") {
            return Err(anyhow::anyhow!("Not a commit object"));
        }

        let content = std::str::from_utf8(&decompressed_data[null_pos + 1..])?;
        Self::parse(content)
    }
}

impl Commit {
    /// Creates a new commit
    pub fn new(
        tree_hash: String,
        parent_hash: Option<String>,
        author: String,
        message: String,
    ) -> Self {
        let timestamp = Utc::now();
        Self {
            tree: tree_hash,
            parent: parent_hash,
            author,
            timestamp,
            message,
        }
    }

    /// Parses commit content into a Commit object
    ///
    /// # Arguments
    ///
    /// * `content` - The raw commit content to parse
    ///
    pub fn parse(content: &str) -> Result<Self> {
        let mut lines = content.lines();
        let mut tree = None;
        let mut parent = None;
        let mut author = None;
        let mut timestamp = None;
        let mut message = Vec::new();
        let mut reading_message = false;

        while let Some(line) = lines.next() {
            if reading_message {
                message.push(line.to_string());
                continue;
            }

            if line.is_empty() {
                reading_message = true;
                continue;
            }

            let (key, value) = line
                .split_once(' ')
                .ok_or_else(|| anyhow::anyhow!("Invalid commit format"))?;
            match key {
                "tree" => tree = Some(value.to_string()),
                "parent" => parent = Some(value.to_string()),
                "author" => {
                    let parts: Vec<&str> = value.rsplitn(2, ' ').collect();
                    author = Some(parts[1].to_string());
                    timestamp = Some(
                        DateTime::from_timestamp(parts[0].parse::<i64>()?, 0)
                            .unwrap()
                            .with_timezone(&Utc),
                    );
                }
                _ => return Err(anyhow::anyhow!("Unknown commit field: {}", key)),
            }
        }

        Ok(Self {
            tree: tree.context("Missing tree hash")?,
            parent,
            author: author.context("Missing author")?,
            timestamp: timestamp.context("Missing timestamp")?,
            message: message.join("\n"),
        })
    }
}

fn parse_identity(s: &str) -> Result<(String, String, chrono::DateTime<chrono::Utc>)> {
    let reg = regex::Regex::new(r"(.*?)<(.*?)> (\d+) ([+-]\d{4})")?;

    let captures = reg
        .captures(s)
        .ok_or_else(|| anyhow!("Invalid identity format"))?;

    let timestamp = captures[3].parse::<i64>()?;
    let tz_offset = captures[4].parse::<i32>()?;

    let dt = chrono::DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| anyhow!("Invalid timestamp"))?
        .with_timezone(&chrono::FixedOffset::east_opt(tz_offset * 60).unwrap());

    Ok((
        captures[1].to_string(),
        captures[2].to_string(),
        dt.to_utc(),
    ))
}

/// Compares two commits and returns the differences between them as a ChangeSet
///
/// This function loads both commits, their associated trees, and computes
/// the differences between all files in those trees.
///
/// # Arguments
///
/// * `from_hash` - The hash of the source commit to compare from
/// * `to_hash` - The hash of the target commit to compare to
/// * `objects_dir` - Path to the objects directory containing commit and tree data
///
/// # Returns
///
/// Returns a [`ChangeSet`] containing all changes between the commits
///
pub fn compare_commits(from_hash: &str, to_hash: &str, objects_dir: &Path) -> Result<ChangeSet> {
    // Load both commits from the object store
    let from_commit = Commit::load(from_hash, objects_dir)
        .with_context(|| format!("Failed to load source commit {}", from_hash))?;
    let to_commit = Commit::load(to_hash, objects_dir)
        .with_context(|| format!("Failed to load target commit {}", to_hash))?;

    // Load the trees referenced by each commit
    let from_tree = read_tree(&from_commit.tree, objects_dir)
        .with_context(|| format!("Failed to load tree {}", from_commit.tree))?;
    let to_tree = read_tree(&to_commit.tree, objects_dir)
        .with_context(|| format!("Failed to load tree {}", to_commit.tree))?;

    // Compare the trees to get the change_set of changes
    let mut change_set = Tree::compare_trees(&from_tree, &to_tree, objects_dir)
        .context("Failed to compare trees")?;

    // Annotate the change_set with commit references
    change_set.set_from(Some(from_hash.to_string()));
    change_set.set_to(Some(to_hash.to_string()));

    Ok(change_set)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_commit_serialization() -> Result<()> {
        let commit = Commit::new(
            "tree-hash".to_string(),
            Some("parent-hash".to_string()),
            "Author <author@example.com>".to_string(),
            "Test message".to_string(),
        );

        let serialized = commit.serialize()?;
        assert!(serialized.len() > 0);
        Ok(())
    }

    #[test]
    fn test_commit_save_load() -> Result<()> {
        let temp_dir = tempdir()?;
        let objects_dir = temp_dir.path().to_path_buf();

        let commit = Commit::new(
            "tree-hash".to_string(),
            Some("parent-hash".to_string()),
            "Author <author@example.com>".to_string(),
            "Test message".to_string(),
        );

        let hash = commit.save(&objects_dir)?;
        let loaded = Commit::load(&hash, &objects_dir)?;

        assert_eq!(commit.tree, loaded.tree);
        assert_eq!(commit.parent, loaded.parent);
        assert_eq!(commit.author, loaded.author);
        assert_eq!(commit.message, loaded.message);

        Ok(())
    }
}
