mod cli;

use clap::Parser;
use cli::{Cli, Commands};
use ncp_core::utils::load_or_init_config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let cfg = load_or_init_config();

    match args.command {
        Some(Commands::Scan {
            host,
            range,
            timeout,
            out,
            udp,
            subnet,
            concurrency,
        }) => {
            let selected_range = if range.is_empty() {
                cfg.default_scan_range
            } else {
                range
            };
            let timeout = if timeout.is_empty() {
                cfg.default_timeout
            } else {
                timeout
            };
            ncp_core::scanner::run(host, selected_range, timeout, out, udp, subnet, concurrency).await?;
        }
        Some(Commands::Send {
            host,
            port,
            path,
            compress,
            encrypt,
            passphrase,
        }) => {
            ncp_core::transfer::send(host, port, path, compress, encrypt, passphrase).await?;
        }
        Some(Commands::Receive {
            port,
            out_dir,
            passphrase,
        }) => {
            ncp_core::transfer::receive(port, out_dir, passphrase).await?;
        }
        Some(Commands::TextSend {
            host,
            port,
            message,
            repeat,
            interval,
        }) => {
            ncp_core::transfer::send_text(host, port, message, repeat, interval).await?;
        }
        Some(Commands::TextListen { port, keep_alive }) => {
            ncp_core::transfer::listen_text(port, keep_alive).await?;
        }
        Some(Commands::Http {
            url,
            method,
            headers,
            header_list,
            body,
            repeat,
            out,
            follow,
            max_redirects,
        }) => {
            ncp_core::http::run(
                url,
                method,
                headers,
                header_list,
                body,
                repeat,
                out,
                follow,
                max_redirects,
            )
            .await?;
        }
        Some(Commands::Sniff {
            interface,
            proto,
            host,
            port,
            dns,
            pcap,
            stats,
        }) => {
            let interface = interface.or(cfg.default_interface);
            ncp_core::sniffer::run(interface, proto, host, port, dns, pcap, stats).await?;
        }
        Some(Commands::Dash) | None => {
            ncp_core::dashboard::run().await?;
        }
    }

    Ok(())
}
