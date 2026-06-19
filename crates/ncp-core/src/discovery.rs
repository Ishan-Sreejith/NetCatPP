use std::process::Command;

#[derive(Debug, Clone)]
pub struct Device {
    pub ip: String,
    pub mac: String,
    pub interface: String,
}

pub fn get_local_devices() -> Vec<Device> {
    let mut devices = Vec::new();

    // Fast ARP lookup (macOS / Linux compatible mostly)
    if let Ok(output) = Command::new("arp").arg("-a").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            // macOS format: ? (192.168.1.1) at 0:11:22:33:44:55 on en0 ifscope [ethernet]
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 && parts[1].starts_with('(') && parts[1].ends_with(')') {
                let ip = parts[1].trim_matches(|c| c == '(' || c == ')').to_string();
                let mac = parts[3].to_string();
                let interface = parts[5].to_string();
                
                // Exclude incomplete/ff addresses
                if mac != "(incomplete)" && mac != "ff:ff:ff:ff:ff:ff" {
                    devices.push(Device { ip, mac, interface });
                }
            }
        }
    }

    devices
}
