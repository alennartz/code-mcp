mod cli;

use clap::Parser;
use cli::{Cli, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate { specs, output } => {
            println!("Generate: {:?} -> {:?}", specs, output);
            todo!("generate command")
        }
        Command::Serve {
            dir,
            transport,
            port,
        } => {
            println!("Serve: {:?} ({} on {})", dir, transport, port);
            todo!("serve command")
        }
        Command::Run {
            specs,
            transport,
            port,
        } => {
            println!("Run: {:?} ({} on {})", specs, transport, port);
            todo!("run command")
        }
    }
}
