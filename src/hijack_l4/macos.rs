use std::{net::SocketAddr, path::PathBuf, time::Duration};

use tokio::task::JoinHandle;
use xshell::{cmd, Shell};

use super::{HijackL4Context, TunMeta};

pub async fn serve(cx: &HijackL4Context) -> anyhow::Result<()> {
    tokio::task::block_in_place(sudo::escalate_if_needed).expect("sudo");

    let tun = TunMeta {
        name: "utun69".into(),
        ip: "198.18.0.69".parse().unwrap(),
    };

    let tun_service = Tun2Socks::new(tun.clone(), cx.local_socks_server, cx.bin.clone());
    let tun_service = tun_service.spawn().await?;
    let mut tun_service_join = tokio::task::JoinSet::new();
    tun_service_join.spawn(tun_service);

    let route = MacosRoute::new(cx.remote_proxy.clone(), tun.clone());
    route.setup().await?;

    tokio::select! {
        res = tokio::signal::ctrl_c() => {
            res?;
            println!("ctrl-c");
        }
        res = tun_service_join.join_next() => {
            let res = res.unwrap();
            let res = res.expect("Thread panicked");
            let res = res.expect("Thread panicked");
            res?;
            println!("tun2socks ended");
        }
    }

    Ok(())
}

struct Tun2Socks {
    tun: TunMeta,
    local_socks_server: SocketAddr,
    bin: PathBuf,
}
impl Tun2Socks {
    pub fn new(tun: TunMeta, local_socks_server: SocketAddr, bin: PathBuf) -> Self {
        Self {
            tun,
            local_socks_server,
            bin,
        }
    }

    pub async fn spawn(&self) -> anyhow::Result<JoinHandle<anyhow::Result<()>>> {
        let bin = self.bin.as_os_str().to_owned();
        let local_socks_server = self.local_socks_server.to_string();
        let tun_name = self.tun.name.to_owned();
        let handle = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let sh = Shell::new().expect("shell unavailable");
            cmd!(
                sh,
                "sudo {bin} -device {tun_name} -proxy socks5://{local_socks_server} -interface en0"
            )
            .run()?;
            Ok(())
        });

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Turn on the interface
        tokio::task::block_in_place(move || {
            let sh = Shell::new().expect("shell unavailable");
            let tun_name = &self.tun.name;
            let tun_ip = self.tun.ip.to_string();
            cmd!(sh, "sudo ifconfig {tun_name} {tun_ip} {tun_ip} up").run()
        })?;

        Ok(handle)
    }
}

struct MacosRoute {
    tun: TunMeta,
    remote_proxy: Vec<std::net::IpAddr>,
}
impl MacosRoute {
    pub fn new(remote_proxy: Vec<std::net::IpAddr>, tun: TunMeta) -> Self
    where
        Self: Sized,
    {
        Self { tun, remote_proxy }
    }

    pub async fn setup(&self) -> anyhow::Result<()> {
        tokio::task::block_in_place(move || {
            let sh = Shell::new().expect("shell unavailable");
            for net in NETS {
                let tun_name = &self.tun.name;
                cmd!(sh, "sudo route add -net {net} -interface {tun_name}").run()?;
            }

            let default_gateway = default_gateway()?;
            for remote_proxy in &self.remote_proxy {
                let remote_proxy = remote_proxy.to_string();
                cmd!(sh, "sudo route add -host {remote_proxy} {default_gateway}").run()?;
            }
            Ok(())
        })
    }
}
impl Drop for MacosRoute {
    /// Clean up routes
    fn drop(&mut self) {
        let sh = Shell::new().expect("shell unavailable");
        for net in NETS {
            let _ = cmd!(sh, "sudo route delete -net {net}").run();
        }
        for remote_proxy in &self.remote_proxy {
            let remote_proxy = remote_proxy.to_string();
            let _ = cmd!(sh, "sudo route delete -host {remote_proxy}").run();
        }
    }
}

fn default_gateway() -> anyhow::Result<String> {
    let sh = Shell::new().expect("shell unavailable");
    let cmd = r#"route -n get default | grep "gateway:" | awk "{print \$2}""#;
    let o = cmd!(sh, "bash -c {cmd}").output()?;
    Ok(String::from_utf8(o.stdout)?)
}

const NETS: [&str; 8] = [
    "1.0.0.0/8",
    "2.0.0.0/7",
    "4.0.0.0/6",
    "8.0.0.0/5",
    "16.0.0.0/4",
    "32.0.0.0/3",
    "64.0.0.0/2",
    "128.0.0.0/1",
];
