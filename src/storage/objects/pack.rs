use crate::storage::objects::blob::Blob;
use crate::storage::objects::commit::Commit;
use crate::storage::objects::tag::Tag;
use crate::storage::objects::tree::Tree;
use crate::storage::utils::{
    OBJ_TYPE_BLOB, OBJ_TYPE_COMMIT, OBJ_TYPE_TAG, OBJ_TYPE_TREE, Object, VoxObject,
};
use anyhow::{Result, anyhow, bail};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use flate2::bufread::ZlibDecoder;
use flate2::{Compression, write::ZlibEncoder};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use super::delta::apply_delta;

/// Represents a packfile containing Vox objects in compressed form
///
/// Packfiles are used to efficiently store and transfer multiple objects
#[derive(Debug)]
pub struct Packfile {
    /// The collection of packed objects
    pub objects: Vec<PackObject>,
    /// Index mapping object hashes to their locations in the packfile
    pub index: HashMap<String, ObjectLocation>,
}

/// Metadata describing an object's physical location within a packfile
#[derive(Debug)]
pub struct ObjectLocation {
    /// Byte offset where the object starts
    pub offset: u64,
    /// Compressed size of the object in bytes
    pub size: u32,
    /// Numerical code indicating the object type
    pub type_code: u8,
}

/// Represents a packed Vox object (either base or delta)
#[derive(Debug)]
pub enum PackObject {
    /// A complete object with its raw data and type
    Base(Vec<u8>, ObjectType),
    /// A delta-compressed object referencing a base object
    Delta {
        /// SHA-1 hash of the base object this delta applies to
        base_hash: String,
        /// Delta instructions needed to reconstruct the object
        data: Vec<u8>,
    },
}

/// Enum of possible object types in the Vox storage system
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectType {
    Commit = 1,
    Tree = 2,
    Blob = 3,
    Tag = 4,
    DeltaRef = 7,
}

impl Packfile {
    /// Creates a new empty packfile
    pub fn new() -> Self {
        Packfile {
            objects: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Adds an object to the packfile
    pub fn add_object(&mut self, obj: &dyn VoxObject) -> Result<()> {
        let obj_type = match obj.object_type() {
            OBJ_TYPE_COMMIT => ObjectType::Commit,
            OBJ_TYPE_TREE => ObjectType::Tree,
            OBJ_TYPE_BLOB => ObjectType::Blob,
            OBJ_TYPE_TAG => ObjectType::Tag,
            _ => bail!("Unsupported object type"),
        };

        let data = obj.serialize()?;
        self.objects.push(PackObject::Base(data, obj_type));
        Ok(())
    }

    /// Serializes the packfile to a byte vector
    pub fn serialize(&mut self) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        // Write packfile header
        buffer.write_all(b"VOXPACK")?;
        buffer.write_u32::<BigEndian>(self.objects.len() as u32)?;

        let mut offset = 12; // Header size (7 magic + 4 byte count)

        for obj in &self.objects {
            let (type_code, content) = match obj {
                PackObject::Base(data, obj_type) => (*obj_type as u8, data.clone()),
                PackObject::Delta { base_hash: _, data } => {
                    (ObjectType::DeltaRef as u8, data.clone())
                }
            };

            // Compress the object data
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
            encoder.write_all(&content)?;
            let compressed = encoder.finish()?;

            let size = compressed.len() as u32;
            let mut header = Vec::new();
            header.write_u8((type_code << 4) | 0x80)?; // Type + MSB flag
            header.write_u24::<BigEndian>(size)?;

            // Compute object hash
            let mut hasher = Sha1::new();
            hasher.update(&content);
            let hash = format!("{:x}", hasher.finalize());

            // Write object to packfile
            buffer.write_all(&header)?;
            buffer.write_all(&compressed)?;

            // Update index
            self.index.insert(
                hash,
                ObjectLocation {
                    offset: offset as u64,
                    size,
                    type_code,
                },
            );

            offset += header.len() + compressed.len();
        }

        Ok(buffer)
    }

    /// Deserializes a packfile from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let mut magic = [0u8; 7];
        cursor.read_exact(&mut magic)?;

        if &magic != b"VOXPACK" {
            bail!("Invalid pack format");
        }

        let object_count = cursor.read_u32::<BigEndian>()?;
        let mut pack = Packfile::new();
        let mut offset = 12;

        for _ in 0..object_count {
            let first_byte = cursor.read_u8()?;
            let type_code = (first_byte >> 4) & 0x07;
            let compressed_size = cursor.read_u24::<BigEndian>()?;

            // Read compressed data
            let mut compressed = vec![0u8; compressed_size as usize];
            cursor.read_exact(&mut compressed)?;

            // Decompress the object
            let mut decoder = ZlibDecoder::new(&compressed[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;

            let obj_type = match type_code {
                1 => ObjectType::Commit,
                2 => ObjectType::Tree,
                3 => ObjectType::Blob,
                4 => ObjectType::Tag,
                7 => ObjectType::DeltaRef,
                _ => bail!("Invalid object type"),
            };

            let (obj, hash) = match obj_type {
                ObjectType::DeltaRef => {
                    // Delta objects store base hash in first 20 bytes
                    let mut base_hash = [0u8; 20];
                    base_hash.copy_from_slice(&decompressed[..20]);
                    let data = decompressed[20..].to_vec();
                    (
                        PackObject::Delta {
                            base_hash: hex::encode(base_hash),
                            data: data.clone(),
                        },
                        hex::encode(Sha1::digest(&data)),
                    )
                }
                _ => {
                    let hash = hex::encode(Sha1::digest(&decompressed));
                    (PackObject::Base(decompressed, obj_type), hash)
                }
            };

            pack.objects.push(obj);
            pack.index.insert(
                hash,
                ObjectLocation {
                    offset,
                    size: compressed_size,
                    type_code,
                },
            );

            offset += 4 + compressed_size as u64; // 1 byte type + 3 byte size
        }

        Ok(pack)
    }

    /// Applies delta compression to reconstruct full objects
    pub fn apply_deltas(&self, base_objects: &HashMap<String, Vec<u8>>) -> Result<Vec<Object>> {
        let mut results = Vec::new();
        for obj in &self.objects {
            match obj {
                PackObject::Base(data, obj_type) => {
                    let obj = Self::parse_object(*obj_type, data)?;
                    results.push(obj);
                }
                PackObject::Delta { base_hash, data } => {
                    let base_data = base_objects
                        .get(base_hash)
                        .ok_or_else(|| anyhow!("Missing base object {}", base_hash))?;

                    let reconstructed = apply_delta(base_data, data)?;
                    let obj_type = Self::detect_type(&reconstructed)?;
                    let obj = Self::parse_object(obj_type, &reconstructed)?;
                    results.push(obj);
                }
            }
        }
        Ok(results)
    }

    /// Parses raw object data into the appropriate Object type
    pub fn parse_object(obj_type: ObjectType, data: &[u8]) -> Result<Object> {
        match obj_type {
            ObjectType::Commit => {
                let commit = Commit::parse(&String::from_utf8(data.to_vec())?);
                Ok(Object::Commit(commit?))
            }
            ObjectType::Tree => {
                let tree = Tree::parse(data)?;
                Ok(Object::Tree(tree))
            }
            ObjectType::Blob => Ok(Object::Blob(Blob {
                data: data.to_vec(),
            })),
            ObjectType::Tag => {
                let tag = Tag::parse(&String::from_utf8(data.to_vec())?)?;
                Ok(Object::Tag(tag))
            }
            _ => bail!("Unsupported object type"),
        }
    }

    /// Detects the object type by examining its content
    pub fn detect_type(data: &[u8]) -> Result<ObjectType> {
        if data.starts_with(b"commit") {
            Ok(ObjectType::Commit)
        } else if data.starts_with(b"tree") {
            Ok(ObjectType::Tree)
        } else if data.starts_with(b"tag") {
            Ok(ObjectType::Tag)
        } else {
            Ok(ObjectType::Blob)
        }
    }
}
