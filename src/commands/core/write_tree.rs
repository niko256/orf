use crate::storage::objects::tree::{create_tree, store_tree};
use anyhow::Result;
use std::path::Path;

pub fn write_tree_command(path: &Path) -> Result<()> {
    let tree = create_tree(path)?;
    let hash = store_tree(&tree)?;
    println!("{}", hash);
    Ok(())
}
