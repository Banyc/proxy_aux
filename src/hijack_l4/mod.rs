use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use clap::Args;

#[cfg(target_os = "macos")]
mod macos;

/// - `tun2socks` download links: <https://github.com/xjasonlyu/tun2socks/releases>
#[derive(Debug, Clone, Args)]
pub struct CmdArgs {
    #[clap(long)]
    /// The entrance of your proxy chain
    pub local_socks_server: SocketAddr,
    #[clap(long)]
    /// Path to the `tun2socks` binary
    pub bin: PathBuf,
}
impl CmdArgs {
    pub async fn run(&self) -> anyhow::Result<()> {
        let cx = HijackL4Context {
            local_socks_server: self.local_socks_server,
            bin: self.bin.clone(),
        };
        serve(&cx).await
    }
}

pub async fn serve(cx: &HijackL4Context) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    macos::serve(cx).await?;

    #[cfg(target_os = "windows")]
    todo!();

    #[cfg(target_os = "linux")]
    todo!();

    Ok(())
}

#[derive(Debug, Clone)]
pub struct HijackL4Context {
    /// The entrance of your proxy chain
    pub local_socks_server: SocketAddr,
    /// Path to the tun-to-socks service binary
    pub bin: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TunMeta {
    pub name: String,
    pub ip: IpAddr,
}
