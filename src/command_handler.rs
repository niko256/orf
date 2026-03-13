use crate::cli::Commands;
use anyhow::Result;
use orf::commands::{
    branching::{branch::branch_command, checkout::checkout_command},
    config::conf_utils::config_command,
    core::{
        add::add_command,
        catfile::cat_file_command,
        commit::commit_command,
        hash_object::{HashObjectArgs, hash_object_command},
        index::{idx_ls::ls_files_command, idx_rm::rm_command},
        init::init_command,
        status::status_command,
        write_tree::write_tree_command,
    },
    history::{diff::diff_command, log::log_command, show::show_command},
    remote::remote::remote_command,
};

pub async fn handle_command(command: Commands) -> Result<()> {
    match command {
        Commands::Init => {
            init_command().await?;
        }
        Commands::CatFile {
            pretty_print,
            object_hash,
            show_type,
            show_size,
        } => {
            cat_file_command(pretty_print, object_hash, show_type, show_size)?;
        }
        Commands::HashObject { file_path } => {
            hash_object_command(HashObjectArgs { file_path })?;
        }
        Commands::Status => {
            status_command()?;
        }
        Commands::LsFiles { stage } => {
            ls_files_command(stage)?;
        }
        Commands::Rm {
            cashed,
            forced,
            paths,
        } => {
            rm_command(&paths, cashed, forced)?;
        }
        Commands::Add { paths } => {
            add_command(&paths)?;
        }
        Commands::WriteTree { path } => {
            write_tree_command(&path)?;
        }
        Commands::Commit { message, author } => {
            commit_command(&message, author)?;
        }
        Commands::Log { count } => {
            log_command(count)?;
        }
        Commands::Show { commit } => {
            show_command(&commit)?;
        }
        Commands::Branch { name, delete, list } => {
            branch_command(name, delete, list)?;
        }
        Commands::Checkout { target, force } => {
            checkout_command(&target, force, None)?;
        }
        Commands::Config { global, config_cmd } => {
            config_command(global, &config_cmd)?;
        }
        Commands::Remote { remote_cmd } => {
            remote_command(&remote_cmd)?;
        }
        Commands::Diff { from, to } => {
            diff_command(from, to)?;
        }
    }
    Ok(())
}
