#[cfg(not(target_os = "android"))]
pub async fn run_capture_internal() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_capture_internal_with_filters(None, None).await
}

#[cfg(not(target_os = "android"))]
pub async fn run_capture_internal_with_filters(
    interface_name: Option<String>,
    proto_filter: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::api::add_packet;
    use pnet::datalink::{self, Channel};
    use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
    use pnet::packet::icmp::IcmpPacket;
    use pnet::packet::ip::IpNextHeaderProtocols;
    use pnet::packet::ipv4::Ipv4Packet;
    use pnet::packet::tcp::{TcpFlags, TcpPacket};
    use pnet::packet::udp::UdpPacket;
    use pnet::packet::Packet;
    use std::net::IpAddr;
    use std::time::{SystemTime, UNIX_EPOCH};

    let interfaces = datalink::interfaces();
    let interface = if let Some(name) = interface_name {
        interfaces
            .into_iter()
            .find(|i| i.name == name)
            .ok_or("Could not find requested interface")?
    } else {
        interfaces
            .into_iter()
            .find(|i| i.is_up() && !i.is_loopback() && !i.ips.is_empty())
            .ok_or("Could not find a suitable default interface")?
    };

    let mut rx = match datalink::channel(&interface, Default::default()) {
        Ok(Channel::Ethernet(_tx, rx)) => rx,
        Ok(_) => return Err("Unhandled channel type".into()),
        Err(e) => return Err(format!("error opening channel: {}", e).into()),
    };

    loop {
        match rx.next() {
            Ok(packet) => {
                if let Some(event) = parse_packet(packet, proto_filter.as_deref()) {
                    add_packet(event);
                }
            }
            Err(_) => continue,
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }

    fn parse_packet(packet: &[u8], proto_filter: Option<&str>) -> Option<crate::api::PacketData> {
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

        let protocol = ipv4.get_next_level_protocol();
        if let Some(filter) = proto_filter {
            let normalized = filter.trim().to_uppercase();
            let allow = match normalized.as_str() {
                "TCP" => protocol == IpNextHeaderProtocols::Tcp,
                "UDP" => protocol == IpNextHeaderProtocols::Udp,
                "ICMP" => protocol == IpNextHeaderProtocols::Icmp,
                _ => true,
            };
            if !allow {
                return None;
            }
        }

        match protocol {
            IpNextHeaderProtocols::Tcp => {
                let tcp = TcpPacket::new(ipv4.payload())?;
                Some(crate::api::PacketData {
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
                Some(crate::api::PacketData {
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
                Some(crate::api::PacketData {
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

        if flags.is_empty() {
            "DATA".to_string()
        } else {
            flags.join(",")
        }
    }
}

#[cfg(target_os = "android")]
pub async fn run_capture_internal() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        tokio::time::sleep(tokio::time::Duration::MAX).await;
    }
}

#[cfg(target_os = "android")]
pub async fn run_capture_internal_with_filters(
    _interface_name: Option<String>,
    _proto_filter: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        tokio::time::sleep(tokio::time::Duration::MAX).await;
    }
}

#[cfg(not(target_os = "android"))]
pub async fn run(
    interface: Option<String>,
    _proto: Option<String>,
    _host: Option<String>,
    _port: Option<u16>,
    _dns: bool,
    _pcap: Option<String>,
    _stats: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use pnet::datalink::{self, Channel};
    use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
    use pnet::packet::ip::IpNextHeaderProtocols;
    use pnet::packet::ipv4::Ipv4Packet;
    use pnet::packet::tcp::{TcpFlags, TcpPacket};
    use pnet::packet::udp::UdpPacket;
    use pnet::packet::Packet;
    use std::net::IpAddr;

    let interface_name = interface.unwrap_or_else(|| {
        datalink::interfaces()
            .into_iter()
            .find(|i| i.is_up() && !i.is_loopback() && !i.ips.is_empty())
            .map(|i| i.name)
            .unwrap_or_else(|| "eth0".to_string())
    });

    let interfaces = datalink::interfaces();
    let iface = interfaces
        .into_iter()
        .find(|i| i.name == interface_name)
        .ok_or_else(|| format!("Interface not found: {}", interface_name))?;

    println!("Starting packet capture on interface: {}", interface_name);
    println!("Press Ctrl+C to stop\n");

    let mut rx = match datalink::channel(&iface, Default::default()) {
        Ok(Channel::Ethernet(_tx, rx)) => rx,
        Ok(_) => return Err("Unhandled channel type".into()),
        Err(e) => return Err(format!("Error opening channel: {}", e).into()),
    };

    let mut packet_count = 0u64;

    loop {
        match rx.next() {
            Ok(packet) => {
                if let Some(event) = parse_packet(packet) {
                    packet_count += 1;
                    println!(
                        "[{}] {}:{} -> {}:{} | {} | {} bytes",
                        packet_count,
                        event.source,
                        event.source_port,
                        event.destination,
                        event.dest_port,
                        event.protocol,
                        event.length
                    );
                }
            }
            Err(e) => {
                eprintln!("Error reading packet: {}", e);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    fn parse_packet(packet: &[u8]) -> Option<crate::api::PacketData> {
        let eth = EthernetPacket::new(packet)?;
        if eth.get_ethertype() != EtherTypes::Ipv4 {
            return None;
        }

        let ipv4 = Ipv4Packet::new(eth.payload())?;
        let src_ip = IpAddr::V4(ipv4.get_source());
        let dst_ip = IpAddr::V4(ipv4.get_destination());
        let length = packet.len();

        match ipv4.get_next_level_protocol() {
            IpNextHeaderProtocols::Tcp => {
                let tcp = TcpPacket::new(ipv4.payload())?;
                Some(crate::api::PacketData {
                    timestamp_ms: 0,
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
                Some(crate::api::PacketData {
                    timestamp_ms: 0,
                    source: src_ip.to_string(),
                    source_port: udp.get_source(),
                    destination: dst_ip.to_string(),
                    dest_port: udp.get_destination(),
                    protocol: "UDP".to_string(),
                    flags: "DATA".to_string(),
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

        if flags.is_empty() {
            "DATA".to_string()
        } else {
            flags.join(",")
        }
    }
}

#[cfg(target_os = "android")]
pub async fn run(
    _interface: Option<String>,
    _proto: Option<String>,
    _host: Option<String>,
    _port: Option<u16>,
    _dns: bool,
    _pcap: Option<String>,
    _stats: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("Packet capture is not supported on Android".into())
}
