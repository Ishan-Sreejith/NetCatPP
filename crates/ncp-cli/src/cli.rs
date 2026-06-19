use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "ncp", about = "NetCat++ - A powerful CLI networking toolkit with a TUI dashboard", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Port scan a host
    Scan {
        host: String,
        #[arg(long, short = 'r', default_value = "1-1000")]
        range: String,
        #[arg(long, default_value = "500ms")]
        timeout: String,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        udp: bool,
        #[arg(long)]
        subnet: Option<String>,
        #[arg(long, default_value_t = 256)]
        concurrency: usize,
    },
    /// Send one file or directory to a remote host
    Send {
        host: String,
        port: u16,
        path: String,
        #[arg(long)]
        compress: bool,
        #[arg(long)]
        encrypt: bool,
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// Listen and receive files
    Receive {
        port: u16,
        #[arg(long)]
        out_dir: Option<String>,
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// Send plain text directly over TCP
    TextSend {
        host: String,
        port: u16,
        message: String,
        #[arg(long, default_value_t = 1)]
        repeat: usize,
        #[arg(long, default_value = "0ms")]
        interval: String,
    },
    /// Listen for incoming plain text over TCP
    TextListen {
        port: u16,
        #[arg(long)]
        keep_alive: bool,
    },
    /// Make an HTTP request
    Http {
        url: String,
        #[arg(long, short = 'X', default_value = "GET")]
        method: String,
        #[arg(long)]
        headers: bool,
        #[arg(long = "header", short = 'H')]
        header_list: Vec<String>,
        #[arg(long)]
        body: Option<String>,
        #[arg(long, default_value_t = 1)]
        repeat: usize,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        follow: bool,
        #[arg(long, default_value_t = 10)]
        max_redirects: usize,
    },
    /// Live packet capture on an interface
    Sniff {
        #[arg(long, short)]
        interface: Option<String>,
        #[arg(long)]
        proto: Option<String>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        dns: bool,
        #[arg(long)]
        pcap: Option<String>,
        #[arg(long)]
        stats: bool,
    },
    /// Open the full TUI dashboard
    Dash,
}
