use clap::{Parser, Subcommand};
use std::path::PathBuf;
use orf::commands::{config::config::ConfigCommands, remote::remote::RemoteCommands};

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Initialize a new orf repository")]
    Init,

    #[command(about = "Provide content or type and size information for repository objects")]
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        #[clap(short = 't')]
        show_type: bool,

        #[clap(short = 's')]
        show_size: bool,

        object_hash: String,
    },

    #[command(about = "Compute object ID and optionally creates a blob from a file")]
    HashObject { file_path: String },

    #[command(about = "Show the working tree status")]
    Status,

    #[command(about = "Remove files from the working tree and from the index")]
    Rm {
        #[clap(long)]
        cashed: bool,

        #[clap(long)]
        forced: bool,

        #[clap(required = true)]
        paths: Vec<PathBuf>,
    },

    #[command(about = "Add file contents to the index")]
    Add {
        #[clap(required_unless_present = "all")]
        paths: Vec<PathBuf>,
    },

    #[command(name = "ls-files", about = "Show information about files in the index")]
    LsFiles {
        #[clap(long)]
        stage: bool,
    },

    #[command(about = "Create a tree object from the current index")]
    WriteTree {
        #[clap(default_value = ".")]
        path: PathBuf,
    },

    #[command(about = "Record changes to the repository")]
    Commit {
        #[clap(short = 'm', long)]
        message: String,

        #[clap(short = 'a', long)]
        author: Option<String>,
    },

    #[command(about = "Show commit logs")]
    Log {
        #[clap(short = 'n', long, default_value = "10")]
        count: usize,
    },

    #[command(about = "Show various types of objects")]
    Show {
        #[clap(default_value = "HEAD")]
        commit: String,
    },

    #[command(about = "List, create, or delete branches")]
    Branch {
        #[clap(help = "Branch name")]
        name: Option<String>,

        #[clap(short, long)]
        delete: bool,

        #[clap(short, long)]
        list: bool,
    },

    Checkout {
        #[clap(help = "Branch name ot commit_hash to checkout")]
        target: String,

        #[clap(
            short,
            long,
            help = "Force checkout even if there are uncommitted changes"
        )]
        force: bool,
    },

    Config {
        #[clap(long, help = "Use global configuration")]
        global: bool,

        #[command(subcommand)]
        config_cmd: ConfigCommands,
    },

    #[command(about = "Manage remote repositories")]
    Remote {
        #[command(subcommand)]
        remote_cmd: RemoteCommands,
    },

    #[command(about = "Show changes between commits, commit and worktree, etc.")]
    Diff {
        #[clap(help = "The commit or reference to compare from")]
        from: Option<String>,

        #[clap(help = "The commit or reference to compare to")]
        to: Option<String>,
    },
}
