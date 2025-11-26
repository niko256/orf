use crate::storage::utils::{OBJ_DIR, OBJ_TYPE_TAG, Storable, VoxObject};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, FixedOffset, Utc};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use regex::Regex;
use sha1::{Digest, Sha1};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

//////////////////////////////////////////////////////////////////////
/// Represents a tag object that points to a specific commit
#[derive(Debug)]
pub struct Tag {
    /// The SHA-1 hash of the object being tagged (usually a commit)
    pub object: String,
    /// The type of object being tagged (typically "commit")
    pub object_type: String,
    /// The name of the tag
    pub tag: String,
    /// Information about who created the tag (name, email, timestamp)
    pub tagger: (String, String, DateTime<Utc>),
    /// The annotation message for the tag
    pub message: String,
}

impl Tag {
    /// Parses a tag object from raw string data
    ///
    /// # Arguments
    /// * `data` - The raw tag object content
    ///
    /// # Format
    /// ```
    /// object <hash>
    /// type <object-type>
    /// tag <name>
    /// tagger <name> <email> <timestamp> <timezone>
    ///
    /// <message>
    /// ```
    pub fn parse(data: &str) -> Result<Self> {
        let lines = data.lines();
        let mut object = None;
        let mut object_type = None;
        let mut tag_name = None;
        let mut tagger = None;
        let mut message = String::new();
        let mut in_message = false;

        for line in lines {
            if in_message {
                message.push_str(line);
                message.push('\n');
                continue;
            }

            if line.is_empty() {
                in_message = true;
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() != 2 {
                continue;
            }

            match parts[0] {
                "object" => object = Some(parts[1].trim().to_string()),
                "type" => object_type = Some(parts[1].trim().to_string()),
                "tag" => tag_name = Some(parts[1].trim().to_string()),
                "tagger" => tagger = Some(Self::parse_identity(parts[1])?),
                _ => {}
            }
        }

        Ok(Tag {
            object: object.ok_or_else(|| anyhow!("Missing object in tag"))?,
            object_type: object_type.ok_or_else(|| anyhow!("Missing object type in tag"))?,
            tag: tag_name.ok_or_else(|| anyhow!("Missing tag name"))?,
            tagger: tagger.ok_or_else(|| anyhow!("Missing tagger"))?,
            message: message.trim().to_string(),
        })
    }

    /// Parses the tagger identity line into components
    ///
    /// # Format
    /// "Name <email> timestamp timezone"
    fn parse_identity(s: &str) -> Result<(String, String, DateTime<Utc>)> {
        let re = Regex::new(r"^(.*) <(.*?)> (\d+) ([\+\-]\d{4})$")?;
        let caps = re
            .captures(s)
            .ok_or_else(|| anyhow!("Invalid tagger format: {}", s))?;

        let name = caps[1].trim().to_string();
        let email = caps[2].trim().to_string();
        let timestamp = caps[3].parse::<i64>()?;
        let timezone_offset = caps[4].parse::<i32>()? * 36; // Convert HHMM to seconds

        let dt = DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| anyhow!("Invalid timestamp: {}", timestamp))?
            .with_timezone(&FixedOffset::east_opt(timezone_offset).unwrap())
            .to_utc();

        Ok((name, email, dt))
    }

    /// Loads a tag object from the object database
    ///
    /// # Arguments
    /// * `hash` - The SHA-1 hash of the tag object
    /// * `objects_dir` - Path to the objects directory
    pub fn load(hash: &str, objects_dir: &Path) -> Result<Self> {
        let dir_path = objects_dir.join(&hash[..2]);
        let object_path = dir_path.join(&hash[2..]);

        // Read and decompress the tag object
        let compressed_data = fs::read(&object_path)
            .with_context(|| format!("Failed to read tag object at {}", object_path.display()))?;

        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;

        // Skip the header (everything before the first null byte)
        let null_pos = decompressed_data
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| anyhow!("Invalid tag object format: missing header terminator"))?;
        let content = &decompressed_data[null_pos + 1..];

        // Parse the tag content
        let content_str = String::from_utf8(content.to_vec())?;
        Self::parse(&content_str)
    }
}

impl VoxObject for Tag {
    /// Returns the object type ("tag")
    fn object_type(&self) -> &str {
        OBJ_TYPE_TAG
    }

    /// Serializes the tag object to bytes
    fn serialize(&self) -> Result<Vec<u8>> {
        let mut content = Vec::new();
        writeln!(content, "object {}", self.object)?;
        writeln!(content, "type {}", self.object_type)?;
        writeln!(content, "tag {}", self.tag)?;
        writeln!(
            content,
            "tagger {} <{}> {} {}",
            self.tagger.0,
            self.tagger.1,
            self.tagger.2.timestamp(),
            self.tagger.2.format("%z")
        )?;
        writeln!(content)?; // Empty line before message
        write!(content, "{}", self.message)?;
        Ok(content)
    }

    /// Computes the SHA-1 hash of the serialized tag
    fn hash(&self) -> Result<String> {
        let mut hasher = Sha1::new();
        hasher.update(&self.serialize()?);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Returns the storage path for this tag in the objects directory
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

impl Storable for Tag {
    /// Saves the tag object to the object database
    fn save(&self, objects_dir: &Path) -> Result<String> {
        let hash = self.hash()?;
        let content = self.serialize()?;

        // Create header with object type and size
        let header = format!("{} {}\0", self.object_type(), content.len());
        let full_content = [header.as_bytes(), &content].concat();

        // Compress the data
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&full_content)?;
        let compressed_data = encoder.finish()?;

        // Write to object database
        let dir_path = objects_dir.join(&hash[..2]);
        fs::create_dir_all(&dir_path)?;
        let object_path = dir_path.join(&hash[2..]);
        fs::write(&object_path, compressed_data)?;

        Ok(hash)
    }
}
