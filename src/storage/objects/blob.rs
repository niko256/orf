use crate::storage::utils::{OBJ_DIR, OBJ_TYPE_BLOB, Storable, VoxObject};
use anyhow::{Context, Result};
use flate2::Compression;
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Represents the blob (binary large object)
/// Blobs store raw file data
pub struct Blob {
    pub data: Vec<u8>,
}

impl Blob {
    /// Creates a new Blob from a file path
    pub fn new(file_path: &str) -> Result<Self> {
        let data = std::fs::read(file_path)?;
        Ok(Blob { data })
    }

    /// Computes the hash pf the file and stores it as a blob object
    pub fn blob_hash(file_path: &str) -> Result<String> {
        let blob = Blob::from_file(file_path)?;
        let object_hash = blob.hash()?;

        // prepare header in the format "blob <size>\0"
        let header = format!("{} {}\0", blob.object_type(), blob.serialize()?.len());

        // Compress header + content
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(header.as_bytes())
            .context("Failed to write header to encoder")?;
        encoder
            .write_all(&blob.serialize()?)
            .context("Failed to write content to encoder")?;

        let compressed_data = encoder.finish().context("Failed to finish compression")?;

        // Store in objects directory with sharded path (first 2 chars of hash as directory)
        let object_path = blob.object_path()?;
        std::fs::create_dir_all(format!("{}/{}", OBJ_DIR.display(), &object_hash[0..2]))
            .context("Failed to create object directory")?;

        let mut object_file = File::create(&object_path).context("Failed to create object file")?;
        object_file
            .write_all(&compressed_data)
            .context("Failed to write compressed data to file")?;

        Ok(object_hash)
    }

    /// Creates a Blob by reading content from the file
    pub fn from_file(file_path: &str) -> Result<Self> {
        // Reading the content from the file
        let mut file = File::open(file_path).context("Failed to open file")?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .context("Failed to read file content")?;

        Ok(Blob { data: content })
    }

    pub fn get_content(&self) -> &Vec<u8> {
        &self.data
    }

    /// Returns a reference to the blob's raw data
    pub fn load(hash: &str, obj_dir: &Path) -> Result<Self> {
        let object_path = obj_dir.join(&hash[0..2]).join(&hash[2..]);
        let compressed = std::fs::read(object_path)?;

        // decompress the Zlib data
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut data = Vec::new();
        decoder.read_to_end(&mut data)?;

        // Find the null byte separator between header and content
        let null_position = data
            .iter()
            .position(|&b| b == 0)
            .context("Invalid blob format: no null byte found")?;

        // extract jyst the content (after the null byte)
        let content = &data[null_position + 1..];

        Ok(Blob {
            data: content.to_vec(),
        })
    }
}

impl VoxObject for Blob {
    /// Returns the type identifier for blob objects ("blob")
    fn object_type(&self) -> &str {
        OBJ_TYPE_BLOB
    }

    /// Serializes the blob data (just returns the raw bytes)
    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(self.get_content().clone())
    }

    /// Computes the SHA-1 hash of the blob's content
    fn hash(&self) -> Result<String> {
        let mut hasher = Sha1::new();
        hasher.update(&self.serialize()?);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Returns the expected storage path for this blob in the object storage
    fn object_path(&self) -> Result<String> {
        let hash = self.hash()?;
        Ok(format!(
            "{}/{}/{}",
            OBJ_DIR.display(),
            &hash[..2], // first 2 chars as dir
            &hash[2..]  // remaining chars as filename
        ))
    }
}

impl Storable for Blob {
    /// Saves the blob to the object storage and returns its hash
    fn save(&self, objects_dir: &Path) -> Result<String> {
        let mut hasher = Sha1::new();
        hasher.update(&self.data);
        let hash = format!("{:x}", hasher.finalize());

        // format the header like (type, size, null byte)
        let header = format!("blob {}\0", self.data.len());
        let full_content = [header.as_bytes(), &self.data].concat();

        // compress the header + content
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&full_content)?;
        let compressed_data = encoder.finish()?;

        // create sharded directory structure and write files
        let dir_path = objects_dir.join(&hash[..2]);
        fs::create_dir_all(&dir_path)?;
        let object_path = dir_path.join(&hash[2..]);
        fs::write(&object_path, compressed_data)?;

        Ok(hash)
    }
}
