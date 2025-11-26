use crate::storage::utils::{Loadable, OBJ_DIR, OBJ_TYPE_CHANGE, VoxObject};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

////////////////////////////////////////////////////////////////

/// Represents a type of change to a tree entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// A file was added
    ADDED {
        /// Path to the new file
        path: PathBuf,
        /// Hash of the new file's content
        new_hash: String,
    },
    /// A file was deleted
    DELETED {
        /// Path to the deleted file
        path: PathBuf,
        /// Hash of the deleted file's content
        old_hash: String,
    },
    /// A file was modified
    MODIFIED {
        /// Path to the modified file
        path: PathBuf,
        /// Hash of the file's previous content
        old_hash: String,
        /// Hash of the file's new content
        new_hash: String,
        /// Summary of changes between versions
        summary: Option<DiffSummary>,
    },
    /// A file was renamed
    RENAMED {
        /// Original path of the file
        old_path: PathBuf,
        /// New path of the file
        new_path: PathBuf,
        /// Hash of the file's content before rename
        old_hash: String,
        /// Hash of the file's content after rename
        new_hash: String,
        /// Summary of changes if content was also modified
        summary: Option<DiffSummary>,
    },
}

/// Summary of changes between two versions of a file
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Number of lines added
    insertions: usize,
    /// Number of lines removed
    removals: usize,
    /// Unified diff text showing changes
    text_diff: Option<String>,
}

/// Collection of changes between two states of a repository
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ChangeSet {
    /// Mapping of paths to their 'change' types
    subchanges: HashMap<PathBuf, ChangeType>,
    /// Optional reference to the "from" state (commit hash, branch name, etc...)
    from: Option<String>,
    /// Optional reference to the "to" state (commit hash, branch name, etc...)
    to: Option<String>,
}

impl ChangeSet {
    /// Creates a new empty ChangeSet with optional references
    pub fn new(from: Option<String>, to: Option<String>) -> Self {
        ChangeSet {
            subchanges: HashMap::new(),
            from,
            to,
        }
    }

    /// Adds a change
    pub fn add_change(&mut self, change: ChangeType) {
        let key = match &change {
            ChangeType::ADDED { path, .. } => path.clone(),
            ChangeType::DELETED { path, .. } => path.clone(),
            ChangeType::RENAMED { new_path, .. } => new_path.clone(),
            ChangeType::MODIFIED { path, .. } => path.clone(),
        };
        self.subchanges.insert(key, change);
    }

    /// Removes a change by path
    pub fn remove_change(&mut self, path: &Path) -> Option<ChangeType> {
        self.subchanges.remove(&path.to_path_buf())
    }

    pub fn is_empty(&self) -> bool {
        self.subchanges.is_empty()
    }

    /// Returns the number of changes
    pub fn len(&self) -> usize {
        self.subchanges.len()
    }

    /// Collects all paths that have changes
    pub fn collect_paths(&self) -> Vec<PathBuf> {
        self.subchanges.keys().cloned().collect()
    }

    /// Gets the change entry for a specific path
    pub fn get_entry(&self, path: &Path) -> Option<&ChangeType> {
        self.subchanges.get(path)
    }

    /// Finds all changes under a specific path prefix
    pub fn find_by_prefix(&self, prefix: &Path) -> HashMap<&PathBuf, &ChangeType> {
        self.subchanges
            .iter()
            .filter(|(path, _)| path.starts_with(prefix))
            .collect()
    }

    pub fn get(&self) -> HashMap<PathBuf, ChangeType> {
        self.subchanges.clone()
    }

    pub fn from_ref(&self) -> Option<&String> {
        self.from.as_ref()
    }

    pub fn to_ref(&self) -> Option<&String> {
        self.to.as_ref()
    }

    pub fn from(&self) -> Option<&str> {
        self.from.as_deref()
    }

    pub fn to(&self) -> Option<&str> {
        self.to.as_deref()
    }

    pub fn set_from(&mut self, from: Option<String>) {
        self.from = from;
    }

    pub fn set_to(&mut self, to: Option<String>) {
        self.to = to;
    }
}

// Getters and setters for DiffSummary
impl DiffSummary {
    /// Creates a new DiffSummary
    pub fn new(insertions: usize, removals: usize, text_diff: Option<String>) -> Self {
        DiffSummary {
            insertions,
            removals,
            text_diff,
        }
    }

    pub fn insertions(&self) -> usize {
        self.insertions
    }

    pub fn removals(&self) -> usize {
        self.removals
    }

    pub fn get_text_diff(&self) -> Option<&String> {
        self.text_diff.as_ref()
    }

    pub fn text_diff(&self) -> Option<&str> {
        self.text_diff.as_deref()
    }

    pub fn set_insertions(&mut self, ins: usize) {
        self.insertions = ins;
    }

    pub fn set_removals(&mut self, rm: usize) {
        self.removals = rm;
    }

    pub fn set_diff(&mut self, text_diff: Option<String>) {
        self.text_diff = text_diff;
    }
}

impl VoxObject for ChangeSet {
    /// Returns the object type ("change")
    fn object_type(&self) -> &str {
        OBJ_TYPE_CHANGE
    }

    /// Serializes the ChangeSet to binary format
    fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serde::encode_to_vec(self, bincode::config::standard())
            .context("Failed to serialize ChangeSet to binary")
    }

    /// Computes the SHA-1 hash of the serialized ChangeSet
    fn hash(&self) -> Result<String> {
        let mut hasher = Sha1::new();
        hasher.update(&VoxObject::serialize(self)?);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Returns the storage path for this ChangeSet in the objects directory
    ///
    /// # Returns
    ///
    /// Returns an error if the hash cannot be computed
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

impl ChangeType {
    pub fn get_path(&self) -> &Path {
        match self {
            ChangeType::ADDED { path, .. } => path,
            ChangeType::DELETED { path, .. } => path,
            ChangeType::MODIFIED { path, .. } => path,
            ChangeType::RENAMED { new_path, .. } => new_path,
        }
    }

    fn get_old_path(&self) -> Option<&PathBuf> {
        match self {
            ChangeType::RENAMED { old_path, .. } => Some(old_path),
            _ => None,
        }
    }

    pub fn path(&self) -> &Path {
        self.get_path().iter().as_path()
    }

    pub fn old_path(&self) -> Option<&Path> {
        self.get_old_path().map(|pb| pb.as_path())
    }

    pub fn get_pathbuf(&self) -> PathBuf {
        self.get_path().to_path_buf()
    }

    pub fn get_old_pathbuf(&self) -> Option<PathBuf> {
        self.get_old_path().cloned()
    }

    pub fn get_new_hash(&self) -> Option<&String> {
        match self {
            ChangeType::ADDED { new_hash, .. } => Some(new_hash),
            ChangeType::MODIFIED { new_hash, .. } => Some(new_hash),
            ChangeType::RENAMED { new_hash, .. } => Some(new_hash),
            _ => None,
        }
    }

    pub fn new_hash(&self) -> Option<&str> {
        self.get_new_hash().map(|s| s.as_str())
    }

    pub fn get_old_hash(&self) -> Option<&String> {
        match self {
            ChangeType::DELETED { old_hash, .. } => Some(old_hash),
            ChangeType::MODIFIED { old_hash, .. } => Some(old_hash),
            ChangeType::RENAMED { old_hash, .. } => Some(old_hash),
            _ => None,
        }
    }

    pub fn old_hash(&self) -> Option<&str> {
        self.get_old_hash().map(|s| s.as_str())
    }

    pub fn get_summary(&self) -> Option<&DiffSummary> {
        match self {
            ChangeType::MODIFIED { summary, .. } => summary.as_ref(),
            ChangeType::RENAMED { summary, .. } => summary.as_ref(),
            _ => None,
        }
    }

    pub fn summary(&self) -> Option<&DiffSummary> {
        self.get_summary()
    }
}

impl Loadable for ChangeSet {
    fn load(hash: &str, objects_dir: &Path) -> Result<Self> {
        let path = objects_dir.join(&hash[..2]).join(&hash[2..]);
        let data = std::fs::read(path)?;

        bincode::serde::decode_from_slice(&data, bincode::config::standard())
            .map(|(result, _)| result)
            .context("Failed to deserialize ChangeSet")
    }
}
