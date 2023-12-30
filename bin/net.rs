use clap::{ColorChoice, Parser, Subcommand};
use futures::future::join_all;
use i3stat::error::Result;
use i3stat::util::route::InterfaceUpdate;
use i3stat::util::{local_block_on, netlink_ipaddr_listen};
use serde_json::json;
use tokio::sync::mpsc;

#[derive(Debug, Parser)]
#[clap(author, version, long_about, name = "i3stat-net", color = ColorChoice::Always)]
/// A command which prints network/80211 information gathered from netlink.
///
/// Each line is a JSON array which contains a list of all interfaces reported
/// by netlink. Wireless interfaces also print 80211 information.
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Copy, Clone, Subcommand)]
enum Command {
    /// Print current network interfaces
    Info,
    /// Watch and print network interfaces whenever a network address change is detected.
    Watch,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let (output, _) = local_block_on(async {
        let (manual_tx, manual_rx) = mpsc::channel(1);
        manual_tx.send(()).await?;

        let mut rx = netlink_ipaddr_listen(manual_rx).await?;

        if let Command::Info = args.command {
            match rx.recv().await {
                Some(interfaces) => print_interfaces(&interfaces).await,
                None => println!("null"),
            }

            return Ok(());
        }

        while let Some(interfaces) = rx.recv().await {
            print_interfaces(&interfaces).await;
        }

        Err("Unexpected end of netlink subscription".into())
    })?;

    output
}

async fn print_interfaces(interfaces: &InterfaceUpdate) {
    println!(
        "{}",
        json!(
            join_all(interfaces.values().map(|interface| async {
                json!({
                    "index": interface.index,
                    "name": interface.name,
                    "mac": interface.mac_address.as_ref().map(|m| m.to_string()),
                    "ips": interface.ip_addresses.iter().collect::<Vec<_>>(),
                    "wireless": interface.wireless_info().await.map(|info| json!({
                        "index": info.index,
                        "interface": info.interface,
                        "mac": info.mac_addr.to_string(),
                        "ssid": info.ssid,
                        "bssid": info.bssid.as_ref().map(|m| m.to_string()),
                        "signal": info.signal.as_ref().map(|s| json!({
                            "dbm": s.dbm,
                            "link": s.link,
                            "quality": s.quality()
                        }))
                    }))
                })
            }))
            .await
        )
    );
}

#[cfg(test)]
#[path = "../src/test_utils.rs"]
mod test_utils;

#[cfg(test)]
crate::gen_manpage!(Cli);
