use crate::storage::objects::branch::Branch;
use anyhow::Result;
use colored::*;

pub fn branch_command(name: Option<String>, delete: bool, _list: bool) -> Result<()> {
    // Handle branch deletion
    if delete {
        if let Some(branch_name) = name {
            // Create branch object for deletion
            let branch = Branch {
                name: branch_name.clone(),
                commit_hash: String::new(), // Empty hash as it's not needed for deletion
            };
            branch.delete()?; // Delete the branch
            println!("Deleted branch '{}'", branch_name.green());
        } else {
            // Error if no branch name provided for deletion
            return Err(anyhow::anyhow!("Branch name required for deletion"));
        }
    } else if let Some(branch_name) = name {
        // Handle branch creation
        // Get current branch to use its commit hash as starting point
        let current_branch =
            Branch::get_current_branch()?.ok_or_else(|| anyhow::anyhow!("No commits yet"))?;

        // Create new branch pointing to current commit
        Branch::new(&branch_name, &current_branch.commit_hash)?;
        println!("Created branch '{}'", branch_name.green());
    } else {
        // Handle branch listing (default behavior)
        let branches = Branch::list()?; // Get all branches
        let current = Branch::get_current_branch()?; // Get current branch for marking

        // Display each branch
        for branch in branches {
            // Show asterisk (*) for current branch, spaces for others
            let prefix = if Some(&branch.name) == current.as_ref().map(|b| &b.name) {
                "* ".green()
            } else {
                "   ".normal()
            };

            // Print branch info: prefix, name, and abbreviated commit hash
            println!(
                "{}{} {}",
                prefix,
                branch.name.green(),
                branch.commit_hash[..7].yellow() // Show first 7 chars of commit hash
            );
        }
    }
    Ok(())
}
