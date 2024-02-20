use clap::{Parser, Subcommand};

mod hijack_l4;

#[derive(Debug, Clone, Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Cmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Cmd {
    HijackL4(hijack_l4::CmdArgs),
}
impl Cmd {
    pub async fn run(&self) -> anyhow::Result<()> {
        match self {
            Cmd::HijackL4(cmd) => cmd.run().await,
        }
    }
}
