use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};
use std::process::Command;
use sysinfo::{ProcessesToUpdate, System};
use tokio::io::AsyncReadExt;

static SYSTEM: LazyLock<Mutex<System>> = LazyLock::new(|| {
    let mut sys = System::new_all();
    sys.refresh_all();
    Mutex::new(sys)
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRequest {
    pub host: String,
    pub range: String,
    pub timeout: String,
    pub udp: bool,
    pub concurrency: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub url: String,
    pub method: String,
    pub body: Option<String>,
    pub headers: Vec<(String, String)>,
    pub repeat: usize,
    pub follow: bool,
    pub max_redirects: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponseData {
    pub status: u16,
    pub final_url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub avg_latency_ms: f64,
    pub total_ms_last_request: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessData {
    pub pid: String,
    pub name: String,
    pub cpu: f32,
    pub memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub cpu_usage: f32,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub memory_pressure: Option<f32>,
    pub gpu_pressure: Option<f32>,
    pub top_processes: Vec<ProcessData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketData {
    pub timestamp_ms: u64,
    pub source: String,
    pub source_port: u16,
    pub destination: String,
    pub dest_port: u16,
    pub protocol: String,
    pub flags: String,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMessage {
    pub timestamp_ms: u64,
    pub peer: String,
    pub message: String,
}

struct PacketCapture {
    packets: VecDeque<PacketData>,
    running: bool,
}

struct TextCapture {
    messages: VecDeque<TextMessage>,
    running: bool,
}

static PACKET_CAPTURE: Mutex<PacketCapture> = Mutex::new(PacketCapture {
    packets: VecDeque::new(),
    running: false,
});

static TEXT_CAPTURE: Mutex<TextCapture> = Mutex::new(TextCapture {
    messages: VecDeque::new(),
    running: false,
});

pub fn scan_host_json(
    host: String,
    range: String,
    timeout: String,
    udp: bool,
    concurrency: usize,
) -> String {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result: crate::scanner::HostInfo = rt.block_on(async {
        match crate::scanner::run(
            host,
            range,
            timeout,
            None,
            udp,
            None,
            concurrency.max(1),
        )
        .await
        {
            Ok(h) => h,
            Err(_) => crate::scanner::HostInfo {
                host: String::new(),
                open_ports: vec![],
                os_guess: String::new(),
            },
        }
    });
    serde_json::to_string(&result).unwrap_or_default()
}

pub fn http_request_json(
    url: String,
    method: String,
    body: Option<String>,
    _repeat: usize,
    follow: bool,
    max_redirects: usize,
) -> String {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = rt.block_on(async {
        let client = match reqwest::Client::builder()
            .redirect(if follow {
                reqwest::redirect::Policy::limited(max_redirects)
            } else {
                reqwest::redirect::Policy::none()
            })
            .build()
        {
            Ok(c) => c,
            Err(_) => return HttpResponseData {
                status: 0,
                final_url: url,
                headers: vec![],
                body: String::new(),
                avg_latency_ms: 0.0,
                total_ms_last_request: 0,
            },
        };
        let method = reqwest::Method::from_bytes(method.to_uppercase().as_bytes()).unwrap_or(reqwest::Method::GET);
        let mut req_builder = client.request(method, &url);
        if let Some(b) = body {
            req_builder = req_builder.body(b);
        }
        match req_builder.send().await {
            Ok(r) => HttpResponseData {
                status: r.status().as_u16(),
                final_url: r.url().to_string(),
                headers: r.headers().iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string())).collect(),
                body: r.text().await.unwrap_or_default(),
                avg_latency_ms: 0.0,
                total_ms_last_request: 0,
            },
            Err(_) => HttpResponseData {
                status: 0,
                final_url: url,
                headers: vec![],
                body: String::new(),
                avg_latency_ms: 0.0,
                total_ms_last_request: 0,
            },
        }
    });
    serde_json::to_string(&result).unwrap_or_default()
}

pub fn send_text(
    host: String,
    port: u16,
    message: String,
    repeat: usize,
    interval: String,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _ = rt.block_on(async {
        crate::transfer::send_text(host, port, message, repeat, interval).await
    });
}

pub fn system_snapshot() -> SystemSnapshot {
    let mut sys = SYSTEM.lock().unwrap();

    sys.refresh_cpu_usage();
    sys.refresh_memory();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let cpu_usage = sys.global_cpu_usage();

    let mut procs: Vec<ProcessData> = sys
        .processes()
        .iter()
        .map(|(pid, p)| ProcessData {
            pid: pid.to_string(),
            name: p.name().to_string_lossy().to_string(),
            cpu: p.cpu_usage(),
            memory_mb: p.memory() / (1024 * 1024),
        })
        .collect();
    procs.sort_by(|a, b| {
        b.cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.memory_mb.cmp(&a.memory_mb))
    });
    procs.truncate(20);

    SystemSnapshot {
        cpu_usage: cpu_usage as f32,
        memory_used_gb: sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        memory_total_gb: sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        memory_pressure: read_memory_pressure_percent(),
        gpu_pressure: read_gpu_pressure_percent(),
        top_processes: procs,
    }
}

fn read_memory_pressure_percent() -> Option<f32> {
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("memory_pressure").output().ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("System-wide memory free percentage:") {
                let free_pct = rest.trim().trim_end_matches('%').parse::<f32>().ok()?;
                return Some((100.0 - free_pct).clamp(0.0, 100.0));
            }
        }
        None
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn read_gpu_pressure_percent() -> Option<f32> {
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("ioreg")
            .args(["-r", "-d", "1", "-c", "IOAccelerator"])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            if line.contains("Device Utilization %") {
                let candidate = line
                    .split('=')
                    .nth(1)
                    .map(str::trim)
                    .unwrap_or("");
                let digits: String = candidate
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if let Ok(v) = digits.parse::<f32>() {
                    if (0.0..=100.0).contains(&v) {
                        return Some(v);
                    }
                }
            }
        }
        None
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn start_capture(interface: Option<String>, proto_filter: Option<String>) {
    let mut capture = PACKET_CAPTURE.lock().unwrap();
    if !capture.running {
        capture.running = true;
        capture.packets.clear();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                if let Err(e) = crate::sniffer::run_capture_internal_with_filters(interface, proto_filter).await {
                    eprintln!("sniffer error: {}", e);
                }
            });
            let mut capture = PACKET_CAPTURE.lock().unwrap();
            capture.running = false;
        });
    }
}

pub fn poll_packets(max_count: usize) -> Vec<PacketData> {
    let mut capture = PACKET_CAPTURE.lock().unwrap();
    let count = capture.packets.len().min(max_count);
    capture.packets.drain(..count).collect()
}

pub fn stop_capture() {
    let mut capture = PACKET_CAPTURE.lock().unwrap();
    capture.running = false;
}

pub fn add_packet(packet: PacketData) {
    let mut capture = PACKET_CAPTURE.lock().unwrap();
    if capture.running {
        if capture.packets.len() >= 1000 {
            capture.packets.pop_front();
        }
        capture.packets.push_back(packet);
    }
}

pub fn start_text_listener(port: u16, keep_alive: bool) {
    let mut capture = TEXT_CAPTURE.lock().unwrap();
    if !capture.running {
        capture.running = true;
        capture.messages.clear();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let addr = format!("0.0.0.0:{}", port);
                let listener = match tokio::net::TcpListener::bind(&addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("text-listen error: {}", e);
                        let mut capture = TEXT_CAPTURE.lock().unwrap();
                        capture.running = false;
                        return;
                    }
                };

                loop {
                    if !TEXT_CAPTURE.lock().unwrap().running {
                        break;
                    }

                    match tokio::time::timeout(std::time::Duration::from_millis(200), listener.accept()).await {
                        Ok(Ok((mut stream, peer))) => {
                            let mut buf = Vec::new();
                            if let Err(e) = stream.read_to_end(&mut buf).await {
                                eprintln!("text-listen read error: {}", e);
                            } else {
                                let msg = String::from_utf8_lossy(&buf).to_string();
                                let timestamp_ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;
                                let mut capture = TEXT_CAPTURE.lock().unwrap();
                                if capture.messages.len() >= 200 {
                                    capture.messages.pop_front();
                                }
                                capture.messages.push_back(TextMessage {
                                    timestamp_ms,
                                    peer: peer.to_string(),
                                    message: msg,
                                });
                            }

                            if !keep_alive {
                                let mut capture = TEXT_CAPTURE.lock().unwrap();
                                capture.running = false;
                                break;
                            }
                        }
                        Ok(Err(e)) => {
                            eprintln!("text-listen accept error: {}", e);
                        }
                        Err(_) => {}
                    }
                }
            });
            let mut capture = TEXT_CAPTURE.lock().unwrap();
            capture.running = false;
        });
    }
}

pub fn poll_text_messages(max_count: usize) -> Vec<TextMessage> {
    let mut capture = TEXT_CAPTURE.lock().unwrap();
    let count = capture.messages.len().min(max_count);
    capture.messages.drain(..count).collect()
}

pub fn stop_text_listener() {
    let mut capture = TEXT_CAPTURE.lock().unwrap();
    capture.running = false;
}
