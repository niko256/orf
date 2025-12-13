use anyhow::{Context, Ok, Result};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

//////////////////////////////////////////

/// Signature for the index file format.
const INDEX_SIGNATURE: &[u8; 4] = b"DIRC";

/// Version of the index file format.
const INDEX_VERSION: u32 = 2;

/// Represents an entry in the index file.
/// Each entry corresponds to a file in the working directory and stores metadata about it.
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub mtime: u64, // Last modification time
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,       // Group ID of the file owner
    pub size: u32,      // Size of the file in bytes
    pub hash: [u8; 20], // SHA-1 hash of the file content
    pub flags: u16,
    pub path: PathBuf, // Path to the file (relative to the repository root)
}

/// Represents the index file, which tracks the state of files in the working directory.
#[derive(Debug, Default)]
pub(crate) struct Index {
    pub entries: HashMap<PathBuf, IndexEntry>, // Map of file paths to their index entries
}

impl IndexEntry {
    /// Creates a new `IndexEntry` for a file at the given path.
    ///
    /// # Arguments
    /// - `path`: The path to the file.
    ///
    /// # Returns
    /// A new `IndexEntry` with metadata populated from the file.
    ///
    pub fn new(path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)?; // Read file metadata

        Ok(IndexEntry {
            mtime: metadata.mtime() as u64, // Last modification time
            dev: metadata.dev() as u32,
            ino: metadata.ino() as u32,
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            size: metadata.size() as u32,
            hash: [0; 20],
            flags: 0,
            path: path.to_path_buf(),
        })
    }
}

impl Index {
    pub fn new() -> Self {
        Index {
            entries: HashMap::new(),
        }
    }

    pub fn add_entry(&mut self, entry: IndexEntry) {
        self.entries.insert(entry.path.clone(), entry);
    }

    pub fn remove_entry(&mut self, path: &Path) -> Option<IndexEntry> {
        self.entries.remove(path)
    }

    /// Retrieves an entry from the index.
    ///
    pub fn get_entry(&self, path: &Path) -> Option<&IndexEntry> {
        let normalized_path = if path.starts_with("./") {
            path.strip_prefix("./").unwrap_or(path) // Normalize path by removing "./"
        } else {
            path
        };
        self.entries.get(normalized_path)
    }

    /// Returns a reference to all entries in the index.
    pub fn get_entries(&self) -> &HashMap<PathBuf, IndexEntry> {
        &self.entries
    }

    /// Writes the index to a file.
    ///
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        // Create the parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory at {:?}", parent))?;
        }

        let mut file = File::create(path)
            .with_context(|| format!("Failed to create index file at {:?}", path))?;

        // Write the index signature and version
        file.write_all(INDEX_SIGNATURE)
            .with_context(|| format!("Failed to write index signature"))?;
        file.write_all(&INDEX_VERSION.to_be_bytes())
            .with_context(|| format!("Failed to write index version"))?;

        // Write the number of entries
        file.write_all(&(self.entries.len() as u32).to_be_bytes())
            .with_context(|| format!("Failed to write entries count"))?;

        // Sort entries by path for consistent ordering
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));

        // Write each entry to the file
        for entry in entries {
            file.write_all(&entry.mtime.to_be_bytes())
                .with_context(|| format!("Failed to write entry mtime"))?;
            file.write_all(&entry.dev.to_be_bytes())
                .with_context(|| format!("Failed to write entry dev"))?;
            file.write_all(&entry.ino.to_be_bytes())
                .with_context(|| format!("Failed to write entry ino"))?;
            file.write_all(&entry.uid.to_be_bytes())
                .with_context(|| format!("Failed to write entry uid"))?;
            file.write_all(&entry.gid.to_be_bytes())
                .with_context(|| format!("Failed to write entry gid"))?;
            file.write_all(&entry.mode.to_be_bytes())
                .with_context(|| format!("Failed to write entry mode"))?;
            file.write_all(&entry.size.to_be_bytes())
                .with_context(|| format!("Failed to write entry size"))?;
            file.write_all(&entry.hash)
                .with_context(|| format!("Failed to write entry hash"))?;
            file.write_all(&entry.flags.to_be_bytes())
                .with_context(|| format!("Failed to write entry flags"))?;

            // Write the file path as a null-terminated string
            let path_str = entry
                .path
                .to_str()
                .with_context(|| format!("Failed to convert path to string"))?;

            file.write_all(path_str.as_bytes())
                .with_context(|| format!("Failed to write entry path"))?;

            file.write_all(&[0])
                .with_context(|| format!("Failed to write path terminator"))?;
        }

        Ok(())
    }

    /// Reads the index from a file.
    ///
    pub fn read_from_file(&mut self, path: &Path) -> Result<()> {
        // Open the index file
        let mut file =
            File::open(path).with_context(|| format!("Failed to open index file at {:?}", path))?;

        // Read and validate the signature
        let mut signature = [0u8; 4];
        file.read_exact(&mut signature)
            .with_context(|| format!("Failed to read index signature"))?;

        if &signature != INDEX_SIGNATURE {
            return Err(anyhow::anyhow!("Invalid index file signature"));
        }

        // Read and validate the version
        let mut version_bytes = [0u8; 4];

        file.read_exact(&mut version_bytes)
            .with_context(|| format!("Failed to read index version"))?;

        let version = u32::from_be_bytes(version_bytes);

        if version != INDEX_VERSION {
            return Err(anyhow::anyhow!("Unsupported index version"));
        }

        // Read the number of entries
        let mut count_bytes = [0u8; 4];
        file.read_exact(&mut count_bytes)?;
        let count = u32::from_be_bytes(count_bytes);

        self.entries.clear();
        for _ in 0..count {
            let mut entry = IndexEntry {
                mtime: 0,
                dev: 0,
                ino: 0,
                mode: 0,
                uid: 0,
                gid: 0,
                size: 0,
                hash: [0; 20],
                flags: 0,
                path: PathBuf::new(),
            };

            // Read metadata fields
            let mut buffer_u64 = [0u8; 8];
            file.read_exact(&mut buffer_u64)?;
            entry.mtime = u64::from_be_bytes(buffer_u64);

            let mut buffer = [0u8; 4];
            file.read_exact(&mut buffer)?;
            entry.dev = u32::from_be_bytes(buffer);

            file.read_exact(&mut buffer)?;
            entry.ino = u32::from_be_bytes(buffer);

            file.read_exact(&mut buffer)?;
            entry.mode = u32::from_be_bytes(buffer);

            file.read_exact(&mut buffer)?;
            entry.uid = u32::from_be_bytes(buffer);

            file.read_exact(&mut buffer)?;
            entry.gid = u32::from_be_bytes(buffer);

            file.read_exact(&mut buffer)?;
            entry.size = u32::from_be_bytes(buffer);

            // Read the SHA-1 hash
            file.read_exact(&mut entry.hash)?;

            // Read the flags
            let mut flag_bytes = [0u8; 2];
            file.read_exact(&mut flag_bytes)?;
            entry.flags = u16::from_be_bytes(flag_bytes);

            // Read the file path
            let mut path_bytes = Vec::new();
            let mut byte = [0u8; 1];
            loop {
                file.read_exact(&mut byte)?;
                if byte[0] == 0 {
                    break;
                }
                path_bytes.push(byte[0]);
            }
            entry.path = PathBuf::from(String::from_utf8_lossy(&path_bytes).into_owned());

            // Add the entry to the index
            self.entries.insert(entry.path.clone(), entry);
        }

        Ok(())
    }
}
