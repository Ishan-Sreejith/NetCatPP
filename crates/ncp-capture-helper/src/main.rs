use pnet::datalink::{self, Channel};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::{TcpFlags, TcpPacket};
use pnet::packet::udp::UdpPacket;
use pnet::packet::icmp::IcmpPacket;
use pnet::packet::Packet;
use serde::Serialize;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
struct PacketData {
    timestamp_ms: u64,
    source: String,
    source_port: u16,
    destination: String,
    dest_port: u16,
    protocol: String,
    flags: String,
    length: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let interface = if let Some(pos) = args.iter().position(|a| a == "--interface") {
        args.get(pos + 1).cloned().unwrap_or_default()
    } else {
        datalink::interfaces()
            .into_iter()
            .find(|i| i.is_up() && !i.is_loopback() && !i.ips.is_empty())
            .map(|i| i.name)
            .unwrap_or_else(|| "en0".to_string())
    };

    let interfaces = datalink::interfaces();
    let iface = interfaces.into_iter()
        .find(|i| i.name == interface)
        .unwrap_or_else(|| {
            eprintln!("Interface not found: {}", interface);
            std::process::exit(1);
        });

    let mut rx = match datalink::channel(&iface, Default::default()) {
        Ok(Channel::Ethernet(_tx, rx)) => rx,
        Ok(_) => {
            eprintln!("Unhandled channel type");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error opening channel: {}", e);
            std::process::exit(1);
        }
    };

    loop {
        match rx.next() {
            Ok(packet) => {
                if let Some(data) = parse_packet(packet) {
                    if let Ok(json) = serde_json::to_string(&data) {
                        println!("{}", json);
                    }
                }
            }
            Err(_) => continue,
        }
    }
}

fn parse_packet(packet: &[u8]) -> Option<PacketData> {
    let eth = EthernetPacket::new(packet)?;
    if eth.get_ethertype() != EtherTypes::Ipv4 {
        return None;
    }

    let ipv4 = Ipv4Packet::new(eth.payload())?;
    let src_ip = IpAddr::V4(ipv4.get_source());
    let dst_ip = IpAddr::V4(ipv4.get_destination());
    let length = packet.len();
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    match ipv4.get_next_level_protocol() {
        IpNextHeaderProtocols::Tcp => {
            let tcp = TcpPacket::new(ipv4.payload())?;
            Some(PacketData {
                timestamp_ms,
                source: src_ip.to_string(),
                source_port: tcp.get_source(),
                destination: dst_ip.to_string(),
                dest_port: tcp.get_destination(),
                protocol: "TCP".to_string(),
                flags: format_tcp_flags(&tcp),
                length,
            })
        }
        IpNextHeaderProtocols::Udp => {
            let udp = UdpPacket::new(ipv4.payload())?;
            Some(PacketData {
                timestamp_ms,
                source: src_ip.to_string(),
                source_port: udp.get_source(),
                destination: dst_ip.to_string(),
                dest_port: udp.get_destination(),
                protocol: "UDP".to_string(),
                flags: "DATA".to_string(),
                length,
            })
        }
        IpNextHeaderProtocols::Icmp => {
            let icmp = IcmpPacket::new(ipv4.payload())?;
            Some(PacketData {
                timestamp_ms,
                source: src_ip.to_string(),
                source_port: 0,
                destination: dst_ip.to_string(),
                dest_port: 0,
                protocol: "ICMP".to_string(),
                flags: format!("type:{} code:{}", icmp.get_icmp_type().0, icmp.get_icmp_code().0),
                length,
            })
        }
        _ => None,
    }
}

fn format_tcp_flags(tcp: &TcpPacket) -> String {
    let mut flags = Vec::new();
    let f = tcp.get_flags();

    if (f & TcpFlags::SYN) != 0 && (f & TcpFlags::ACK) != 0 {
        flags.push("SYN-ACK");
    } else if (f & TcpFlags::SYN) != 0 {
        flags.push("SYN");
    }
    if (f & TcpFlags::ACK) != 0 {
        flags.push("ACK");
    }
    if (f & TcpFlags::PSH) != 0 {
        flags.push("PSH");
    }
    if (f & TcpFlags::FIN) != 0 {
        flags.push("FIN");
    }
    if (f & TcpFlags::RST) != 0 {
        flags.push("RST");
    }

    if flags.is_empty() { "DATA".to_string() } else { flags.join(",") }
}
