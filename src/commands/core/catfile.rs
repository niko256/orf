use crate::storage::utils::{OBJ_DIR, OBJ_TYPE_BLOB, OBJ_TYPE_TREE};
use crate::storage::utils::{Object, VoxObject};
use anyhow::bail;
use anyhow::{Context, Result};
use flate2::read::ZlibDecoder;
use std::str::FromStr;
use std::{fs::File, io::Read};

/////////////////////////////////

const HASH_PREFIX_LEN: usize = 2;
const HASH_BYTES_LEN: usize = 20;

struct Entry<'a> {
    mode: &'a str,
    name: &'a str,
    hash: String,
}

pub fn cat_file_command(
    pretty_print: bool,
    object_hash: String,
    show_type: bool,
    show_size: bool,
) -> Result<()> {
    if object_hash.is_empty() {
        bail!("Empty object hash provided!");
    }

    let object_data = read_vox_object(&object_hash)?;
    let (object_type, content) = parse_object_header(&object_data)?;

    match (show_type, show_size, pretty_print) {
        (true, false, false) => display_type(&object_type),
        (false, true, false) => display_size(content),
        _ => display_all(&object_type, content)?,
    }

    Ok(())
}

fn read_vox_object(hash: &str) -> Result<Vec<u8>> {
    let object_path = format!(
        "{}/{}/{}",
        OBJ_DIR.display(),
        &hash[..HASH_PREFIX_LEN],
        &hash[HASH_PREFIX_LEN..]
    );

    let file = File::open(&object_path)
        .with_context(|| format!("Failed to open object file: {}", hash))?;

    let mut decoder = ZlibDecoder::new(file);
    let mut data = Vec::new();
    decoder
        .read_to_end(&mut data)
        .context("Failed to read object data")?;

    Ok(data)
}

fn parse_object_header(data: &[u8]) -> Result<(Object, &[u8])> {
    let header_end = data
        .iter()
        .position(|&b| b == b'\0')
        .context("Failed to find header end")?;

    let header = String::from_utf8_lossy(&data[..header_end]);
    let object_type = header
        .split(' ')
        .next()
        .map(Object::from_str)
        .unwrap_or(Ok(Object::Unknown("unknown".to_string())));

    Ok((object_type?, &data[header_end + 1..]))
}

fn display_type(object_type: &Object) {
    println!("{}", object_type.object_type());
}

fn display_size(content: &[u8]) {
    println!("{}", content.len());
}

fn display_tree_content(data: &[u8]) -> Result<()> {
    let mut pos = 0;
    while pos < data.len() {
        let entry = parse_tree_entry(&data[pos..]).context("Failed to parse tree entry")?;

        println!(
            "{} {} {}\t{}",
            entry.mode,
            if entry.mode.starts_with("40") {
                OBJ_TYPE_TREE
            } else {
                OBJ_TYPE_BLOB
            },
            entry.hash,
            entry.name
        );

        pos += entry.name.len() + entry.mode.len() + HASH_BYTES_LEN + 2; // +2 for null byte and space
    }
    Ok(())
}

fn parse_tree_entry(data: &[u8]) -> Result<Entry> {
    let null_pos = data
        .iter()
        .position(|&b| b == 0)
        .context("Invalid format: no null byte found in entry")?;

    let entry_meta = std::str::from_utf8(&data[..null_pos])?;
    let (mode, name) = entry_meta
        .split_once(' ')
        .context("Invalid format: no space in entry metadata")?;

    let hash_start = null_pos + 1;
    let hash_end = hash_start + HASH_BYTES_LEN;
    let hash = hex::encode(&data[hash_start..hash_end]);

    Ok(Entry { mode, name, hash })
}

fn display_all(object_type: &Object, content: &[u8]) -> Result<()> {
    display_type(object_type);
    display_size(content);
    Ok(())
}
