use futures::stream::{self, StreamExt};
use ipnet::Ipv4Net;
use serde::Serialize;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::Command;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::timeout;

use crate::utils::parse_timeout_duration;

#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub port: u16,
    pub service: String,
    pub status: String,
    pub banner: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostInfo {
    pub host: String,
    pub open_ports: Vec<ScanResult>,
    pub os_guess: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SweepResult {
    pub subnet: String,
    pub live_hosts: Vec<String>,
    pub duration_ms: u128,
}

pub async fn run(
    host: String,
    range: String,
    timeout_str: String,
    out: Option<String>,
    udp: bool,
    subnet: Option<String>,
    concurrency: usize,
) -> Result<HostInfo, Box<dyn std::error::Error>> {
    let timeout = parse_timeout_duration(&timeout_str)?;

    if let Some(cidr) = subnet {
        let sweep = ping_sweep(&cidr, concurrency.max(1)).await?;
        println!("Ping sweep on {}", cidr);
        for ip in &sweep.live_hosts {
            println!("  {} up", ip);
        }
        println!("\n  {} live hosts in {:.2?}", sweep.live_hosts.len(), Duration::from_millis(sweep.duration_ms as u64));

        if let Some(path) = out {
            write_sweep_output(&path, &sweep)?;
            println!("  Saved results to {}", path);
        }

        return Ok(HostInfo {
            host,
            open_ports: Vec::new(),
            os_guess: "N/A (subnet sweep)".to_string(),
        });
    }

    let ports = parse_range(&range)?;
    let port_desc = if range.contains(',') {
        format!("[{}]", ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(","))
    } else {
        format!("{}-{}", ports.first().unwrap_or(&1), ports.last().unwrap_or(&1000))
    };
    println!("Scanning {} (ports {})", host, port_desc);
    println!("Mode: {}", if udp { "UDP" } else { "TCP" });

    let os_guess = detect_os(&host);
    println!("OS Guess: {}", os_guess);

    let started = Instant::now();
    let scan_target = host.clone();
    let mut open_ports: Vec<ScanResult> = Vec::new();

    let mut stream = stream::iter(ports)
        .map(|port| {
            let host = scan_target.clone();
            async move {
                if udp {
                    udp_probe(host, port, timeout).await
                } else {
                    tcp_probe(host, port, timeout).await
                }
            }
        })
        .buffer_unordered(concurrency.max(1));

    while let Some(entry) = stream.next().await {
        if let Some(scan) = entry {
            let banner = scan.banner.as_deref().unwrap_or("");
            println!("  {:<5} {:<10} ██ {:<13} {}", scan.port, scan.service, scan.status, banner);
            open_ports.push(scan);
        }
    }

    open_ports.sort_by_key(|r| r.port);
    let elapsed = started.elapsed();
    println!("\n  {} matching ports found in {:.2?}", open_ports.len(), elapsed);

    if let Some(path) = out {
        write_scan_output(&path, &host, &os_guess, &open_ports, elapsed)?;
        println!("  Saved results to {}", path);
    }

    Ok(HostInfo {
        host,
        open_ports,
        os_guess,
    })
}

async fn tcp_probe(host: String, port: u16, timeout_dur: Duration) -> Option<ScanResult> {
    let addr = format!("{}:{}", host, port);
    match timeout(timeout_dur, TcpStream::connect(&addr)).await {
        Ok(Ok(mut stream)) => {
            let banner = grab_banner(&mut stream).await;
            Some(ScanResult {
                port,
                service: service_name(port).to_string(),
                status: "open".to_string(),
                banner,
            })
        }
        _ => None,
    }
}

async fn udp_probe(host: String, port: u16, timeout_dur: Duration) -> Option<ScanResult> {
    let Ok(ip) = IpAddr::from_str(&host) else {
        return None;
    };

    let socket = UdpSocket::bind("0.0.0.0:0").await.ok()?;
    let target = SocketAddr::new(ip, port);
    if socket.send_to(&[0], target).await.is_err() {
        return None;
    }

    let mut buf = [0_u8; 2048];
    match timeout(timeout_dur, socket.recv_from(&mut buf)).await {
        Ok(Ok((_n, _from))) => Some(ScanResult {
            port,
            service: service_name(port).to_string(),
            status: "open".to_string(),
            banner: Some("UDP response".to_string()),
        }),
        _ => None,
    }
}

async fn grab_banner(stream: &mut TcpStream) -> Option<String> {
    let mut buf = [0_u8; 256];
    if let Ok(Ok(n)) = timeout(Duration::from_millis(400), stream.read(&mut buf)).await {
        if n > 0 {
            let line = String::from_utf8_lossy(&buf[..n]).replace(['\r', '\n'], " ");
            return Some(line.trim().to_string());
        }
    }

    let _ = stream.write_all(b"HEAD / HTTP/1.0\r\n\r\n").await;
    if let Ok(Ok(n)) = timeout(Duration::from_millis(400), stream.read(&mut buf)).await {
        if n > 0 {
            let body = String::from_utf8_lossy(&buf[..n]);
            if let Some(server) = body
                .lines()
                .find(|line| line.to_ascii_lowercase().starts_with("server:"))
            {
                return Some(server.to_string());
            }
        }
    }

    None
}

fn detect_os(host: &str) -> String {
    let output = if cfg!(target_os = "windows") {
        Command::new("ping").args(["-n", "1", host]).output()
    } else {
        Command::new("ping").args(["-c", "1", "-t", "1", host]).output()
    };

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        if let Some(ttl_start) = text.find("ttl=") {
            let slice = &text[(ttl_start + 4)..];
            let end = slice
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(slice.len());
            if let Ok(ttl) = slice[..end].parse::<u8>() {
                return match ttl {
                    0..=64 => format!("Linux/Unix-like (ttl={ttl})"),
                    65..=128 => format!("Windows-like (ttl={ttl})"),
                    _ => format!("Network device/macOS-like (ttl={ttl})"),
                };
            }
        }
    }

    "Unknown".to_string()
}

async fn ping_sweep(cidr: &str, concurrency: usize) -> Result<SweepResult, Box<dyn std::error::Error>> {
    let net: Ipv4Net = cidr.parse()?;
    let started = Instant::now();

    let hosts: Vec<Ipv4Addr> = net.hosts().collect();
    let mut stream = stream::iter(hosts)
        .map(|ip| async move {
            if ping_host(ip) {
                Some(ip.to_string())
            } else {
                None
            }
        })
        .buffer_unordered(concurrency.max(1));

    let mut live_hosts = Vec::new();
    while let Some(host) = stream.next().await {
        if let Some(host) = host {
            live_hosts.push(host);
        }
    }

    live_hosts.sort();

    Ok(SweepResult {
        subnet: cidr.to_string(),
        live_hosts,
        duration_ms: started.elapsed().as_millis(),
    })
}

fn ping_host(ip: Ipv4Addr) -> bool {
    let ip_s = ip.to_string();
    let result = if cfg!(target_os = "windows") {
        Command::new("ping").args(["-n", "1", "-w", "900", &ip_s]).output()
    } else {
        Command::new("ping")
            .args(["-c", "1", "-W", "1", &ip_s])
            .output()
    };

    result.map(|o| o.status.success()).unwrap_or(false)
}

fn parse_range(s: &str) -> Result<Vec<u16>, String> {
    if s.contains(',') {
        let ports: Vec<u16> = s
            .split(',')
            .map(|p| p.trim().parse::<u16>().map_err(|_| "Invalid port range".to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(ports);
    }

    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 2 {
        let start: u16 = parts[0].parse().map_err(|_| "Invalid port range")?;
        let end: u16 = parts[1].parse().map_err(|_| "Invalid port range")?;
        if start > end {
            return Err("Invalid range: start must be <= end".to_string());
        }
        Ok((start..=end).collect())
    } else if parts.len() == 1 {
        let single: u16 = s.parse().map_err(|_| "Invalid port range")?;
        Ok(vec![single])
    } else {
        Err("Invalid port range".to_string())
    }
}

fn service_name(port: u16) -> &'static str {
    match port {
        20 | 21 => "FTP",
        22 => "SSH",
        23 => "Telnet",
        25 => "SMTP",
        53 => "DNS",
        80 => "HTTP",
        110 => "POP3",
        143 => "IMAP",
        443 => "HTTPS",
        3306 => "MySQL",
        5432 => "PostgreSQL",
        6379 => "Redis",
        8080 => "HTTP-ALT",
        _ => "-",
    }
}

fn write_scan_output(
    path: &str,
    host: &str,
    os_guess: &str,
    open_ports: &[ScanResult],
    duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.to_lowercase().ends_with(".csv") {
        let mut csv = String::from("host,os_guess,port,service,status,banner\n");
        for row in open_ports {
            let banner = row.banner.as_deref().unwrap_or("").replace('"', "\"\"");
            csv.push_str(&format!(
                "{},{},{},{},{},\"{}\"\n",
                host, os_guess, row.port, row.service, row.status, banner
            ));
        }
        std::fs::write(path, csv)?;
        return Ok(());
    }

    let payload = json!({
        "host": host,
        "os_guess": os_guess,
        "duration_ms": duration.as_millis(),
        "open_ports": open_ports,
        "count": open_ports.len(),
    });
    std::fs::write(path, serde_json::to_vec_pretty(&payload)?)?;
    Ok(())
}

fn write_sweep_output(path: &str, sweep: &SweepResult) -> Result<(), Box<dyn std::error::Error>> {
    if path.to_lowercase().ends_with(".csv") {
        let mut csv = String::from("subnet,host\n");
        for host in &sweep.live_hosts {
            csv.push_str(&format!("{},{}\n", sweep.subnet, host));
        }
        std::fs::write(path, csv)?;
        return Ok(());
    }

    std::fs::write(path, serde_json::to_vec_pretty(sweep)?)?;
    Ok(())
}
