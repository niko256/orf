mod cli;
mod command_handler;

use clap::Parser;
use cli::Cli;

use command_handler::handle_command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    handle_command(cli.command).await?;
    Ok(())
}
