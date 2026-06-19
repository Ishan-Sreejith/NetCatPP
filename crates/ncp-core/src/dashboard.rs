use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Sparkline, Wrap},
    Frame, Terminal,
};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io;
use std::net::IpAddr;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use sysinfo::{ProcessesToUpdate, System};
use tokio::sync::mpsc;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

#[cfg(not(target_os = "android"))]
use pnet::datalink::{self, Channel::Ethernet};
#[cfg(not(target_os = "android"))]
use pnet::packet::Packet;
#[cfg(not(target_os = "android"))]
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
#[cfg(not(target_os = "android"))]
use pnet::packet::icmp::IcmpPacket;
#[cfg(not(target_os = "android"))]
use pnet::packet::ip::IpNextHeaderProtocols;
#[cfg(not(target_os = "android"))]
use pnet::packet::ipv4::Ipv4Packet;
#[cfg(not(target_os = "android"))]
use pnet::packet::tcp::TcpPacket;
#[cfg(not(target_os = "android"))]
use pnet::packet::udp::UdpPacket;

#[derive(Debug, Clone)]
struct InterfaceInfo {
    name: String,
    ips: Vec<IpAddr>,
}

#[derive(Debug, Clone)]
struct PacketRow {
    time: String,
    proto: String,
    src: String,
    dst: String,
    bytes: u64,
}

#[derive(Debug, Clone)]
struct ConnectionRow {
    proto: String,
    endpoint: String,
    state: String,
    bytes: u64,
}

#[derive(Debug, Clone)]
struct ProcessRow {
    pid: String,
    name: String,
    cpu: f32,
    mem_mb: u64,
}

#[derive(Debug, Clone, Copy)]
enum FocusPanel {
    Interfaces,
    Graph,
    Packets,
    Hosts,
    Connections,
}

impl FocusPanel {
    fn next(self) -> Self {
        match self {
            FocusPanel::Interfaces => FocusPanel::Graph,
            FocusPanel::Graph => FocusPanel::Packets,
            FocusPanel::Packets => FocusPanel::Hosts,
            FocusPanel::Hosts => FocusPanel::Connections,
            FocusPanel::Connections => FocusPanel::Interfaces,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DirectionKind {
    Upload,
    Download,
    Other,
}

#[derive(Debug, Clone)]
struct PacketEvent {
    proto: String,
    src: String,
    dst: String,
    size: u64,
    state: String,
    endpoint: String,
    remote_host: String,
    direction: DirectionKind,
}

enum DashEvent {
    Packet(PacketEvent),
    Error(String),
}

struct CaptureWorker {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl CaptureWorker {
    fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct App {
    paused: bool,
    show_help: bool,
    sorting_desc: bool,
    filter_mode: bool,
    filter_input: Input,
    focus: FocusPanel,

    interfaces: Vec<InterfaceInfo>,
    interface_state: ListState,

    packets: Vec<PacketRow>,
    upload_hist: Vec<u64>,
    download_hist: Vec<u64>,
    host_totals: HashMap<String, u64>,
    connection_map: HashMap<String, ConnectionRow>,

    event_tx: mpsc::UnboundedSender<DashEvent>,
    event_rx: mpsc::UnboundedReceiver<DashEvent>,
    capture: Option<CaptureWorker>,

    pending_upload: u64,
    pending_download: u64,
    last_rate_tick: Instant,
    last_error: Option<String>,

    sys: System,
    cpu_usage: f32,
    mem_used: u64,
    mem_total: u64,
    gpu_usage: Option<f32>,
    top_processes: Vec<ProcessRow>,
    last_sys_tick: Instant,
    last_gpu_tick: Instant,
}

impl App {
    fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let interfaces: Vec<InterfaceInfo> = list_interfaces();
        let mut interface_state = ListState::default();
        if !interfaces.is_empty() {
            interface_state.select(Some(0));
        }

        let mut sys = System::new_all();
        sys.refresh_all();

        Self {
            paused: false,
            show_help: false,
            sorting_desc: true,
            filter_mode: false,
            filter_input: Input::default(),
            focus: FocusPanel::Interfaces,
            interfaces,
            interface_state,
            packets: Vec::new(),
            upload_hist: vec![0; 60],
            download_hist: vec![0; 60],
            host_totals: HashMap::new(),
            connection_map: HashMap::new(),
            event_tx,
            event_rx,
            capture: None,
            pending_upload: 0,
            pending_download: 0,
            last_rate_tick: Instant::now(),
            last_error: None,
            sys,
            cpu_usage: 0.0,
            mem_used: 0,
            mem_total: 0,
            gpu_usage: None,
            top_processes: Vec::new(),
            last_sys_tick: Instant::now(),
            last_gpu_tick: Instant::now() - Duration::from_secs(10),
        }
    }

    fn selected_interface(&self) -> Option<&InterfaceInfo> {
        let idx = self.interface_state.selected()?;
        self.interfaces.get(idx)
    }

    fn selected_interface_name(&self) -> String {
        self.selected_interface()
            .map(|i| i.name.clone())
            .unwrap_or_else(|| "n/a".to_string())
    }

    fn next_interface(&mut self) {
        if self.interfaces.is_empty() {
            return;
        }
        let next = match self.interface_state.selected() {
            Some(i) if i + 1 < self.interfaces.len() => i + 1,
            _ => 0,
        };
        self.interface_state.select(Some(next));
    }

    fn prev_interface(&mut self) {
        if self.interfaces.is_empty() {
            return;
        }
        let prev = match self.interface_state.selected() {
            Some(0) | None => self.interfaces.len() - 1,
            Some(i) => i - 1,
        };
        self.interface_state.select(Some(prev));
    }

    fn restart_capture(&mut self) {
        self.stop_capture();
        self.last_error = None;

        let Some(selected) = self.selected_interface() else {
            self.last_error = Some("No network interface available".to_string());
            return;
        };

        match spawn_capture_worker(
            selected.name.clone(),
            selected.ips.clone(),
            self.event_tx.clone(),
        ) {
            Ok(worker) => {
                self.capture = Some(worker);
            }
            Err(err) => {
                self.last_error = Some(err);
            }
        }
    }

    fn stop_capture(&mut self) {
        if let Some(worker) = self.capture.take() {
            worker.stop();
        }
    }

    fn drain_events(&mut self) {
        while let Ok(ev) = self.event_rx.try_recv() {
            match ev {
                DashEvent::Packet(pkt) => {
                    if self.paused {
                        continue;
                    }
                    self.apply_packet(pkt);
                }
                DashEvent::Error(err) => {
                    self.last_error = Some(err);
                }
            }
        }
    }

    fn apply_packet(&mut self, pkt: PacketEvent) {
        let row = PacketRow {
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
            proto: pkt.proto.clone(),
            src: pkt.src.clone(),
            dst: pkt.dst.clone(),
            bytes: pkt.size,
        };
        self.packets.push(row);
        if self.packets.len() > 250 {
            self.packets.remove(0);
        }

        match pkt.direction {
            DirectionKind::Upload => self.pending_upload += pkt.size,
            DirectionKind::Download => self.pending_download += pkt.size,
            DirectionKind::Other => {}
        }

        *self.host_totals.entry(pkt.remote_host.clone()).or_insert(0) += pkt.size;
        if self.host_totals.len() > 500 {
            prune_hosts(&mut self.host_totals);
        }

        let key = format!("{}|{}", pkt.proto, pkt.endpoint);
        let entry = self
            .connection_map
            .entry(key)
            .or_insert_with(|| ConnectionRow {
                proto: pkt.proto.clone(),
                endpoint: pkt.endpoint.clone(),
                state: pkt.state.clone(),
                bytes: 0,
            });
        entry.bytes += pkt.size;
        entry.state = pkt.state;

        if self.connection_map.len() > 250 {
            prune_connections(&mut self.connection_map);
        }
    }

    fn update_rate_history(&mut self) {
        if self.last_rate_tick.elapsed() < Duration::from_secs(1) {
            return;
        }

        self.upload_hist.remove(0);
        self.download_hist.remove(0);
        self.upload_hist.push(self.pending_upload);
        self.download_hist.push(self.pending_download);
        self.pending_upload = 0;
        self.pending_download = 0;
        self.last_rate_tick = Instant::now();
    }

    fn update_system_metrics(&mut self) {
        if self.last_sys_tick.elapsed() < Duration::from_secs(1) {
            return;
        }

        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);

        self.cpu_usage = self.sys.global_cpu_usage();
        self.mem_used = self.sys.used_memory();
        self.mem_total = self.sys.total_memory();

        let mut procs: Vec<ProcessRow> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, p)| ProcessRow {
                pid: pid.to_string(),
                name: p.name().to_string_lossy().to_string(),
                cpu: p.cpu_usage(),
                mem_mb: p.memory() / (1024 * 1024),
            })
            .collect();

        procs.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
        procs.truncate(7);
        self.top_processes = procs;
        self.last_sys_tick = Instant::now();

        if self.last_gpu_tick.elapsed() >= Duration::from_secs(5) {
            self.gpu_usage = read_gpu_usage_percent();
            self.last_gpu_tick = Instant::now();
        }
    }
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.restart_capture();
    let res = run_app(&mut terminal, &mut app).await;
    app.stop_capture();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let mut last_draw = Instant::now();
    let draw_rate = Duration::from_millis(120);

    loop {
        app.drain_events();
        app.update_rate_history();
        app.update_system_metrics();

        terminal.draw(|f| ui(f, app))?;

        let timeout = draw_rate
            .checked_sub(last_draw.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.filter_mode {
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => app.filter_mode = false,
                            _ => {
                                let _ = app.filter_input.handle_event(&Event::Key(key));
                            }
                        }
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Tab => app.focus = app.focus.next(),
                        KeyCode::Char('p') => app.paused = !app.paused,
                        KeyCode::Char('?') => app.show_help = !app.show_help,
                        KeyCode::Char('f') => app.filter_mode = true,
                        KeyCode::Char('s') => app.sorting_desc = !app.sorting_desc,
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.next_interface();
                            app.restart_capture();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.prev_interface();
                            app.restart_capture();
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_draw.elapsed() >= draw_rate {
            last_draw = Instant::now();
        }
    }
}

#[cfg(not(target_os = "android"))]
fn list_interfaces() -> Vec<InterfaceInfo> {
    let mut interfaces: Vec<InterfaceInfo> = datalink::interfaces()
        .into_iter()
        .filter(|i| i.is_up() && !i.is_loopback() && !i.ips.is_empty())
        .map(|i| InterfaceInfo {
            name: i.name,
            ips: i.ips.into_iter().map(|n| n.ip()).collect(),
        })
        .collect();

    if interfaces.is_empty() {
        interfaces = datalink::interfaces()
            .into_iter()
            .map(|i| InterfaceInfo {
                name: i.name,
                ips: i.ips.into_iter().map(|n| n.ip()).collect(),
            })
            .collect();
    }
    interfaces
}

#[cfg(target_os = "android")]
fn list_interfaces() -> Vec<InterfaceInfo> {
    Vec::new()
}

#[cfg(not(target_os = "android"))]
fn spawn_capture_worker(
    iface_name: String,
    local_ips: Vec<IpAddr>,
    tx: mpsc::UnboundedSender<DashEvent>,
) -> Result<CaptureWorker, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = stop.clone();

    let handle = std::thread::spawn(move || {
        let iface = match datalink::interfaces()
            .into_iter()
            .find(|i| i.name == iface_name)
        {
            Some(i) => i,
            None => {
                let _ = tx.send(DashEvent::Error("Selected interface not found".to_string()));
                return;
            }
        };

        let config = datalink::Config {
            read_timeout: Some(Duration::from_millis(250)),
            ..Default::default()
        };

        let (_, mut rx) = match datalink::channel(&iface, config) {
            Ok(Ethernet(tx_eth, rx_eth)) => (tx_eth, rx_eth),
            Ok(_) => {
                let _ = tx.send(DashEvent::Error("Unsupported datalink channel".to_string()));
                return;
            }
            Err(err) => {
                let _ = tx.send(DashEvent::Error(format!(
                    "Capture error on {}: {}. Try: sudo ncp dash",
                    iface.name, err
                )));
                return;
            }
        };

        let local_set: HashSet<IpAddr> = local_ips.into_iter().collect();

        while !stop_signal.load(Ordering::Relaxed) {
            match rx.next() {
                Ok(packet) => {
                    if let Some(parsed) = parse_packet(packet, &local_set) {
                        let _ = tx.send(DashEvent::Packet(parsed));
                    }
                }
                Err(err) => {
                    let msg = err.to_string().to_lowercase();
                    if msg.contains("timed out") || msg.contains("timeout") {
                        continue;
                    }
                    if !stop_signal.load(Ordering::Relaxed) {
                        let _ = tx.send(DashEvent::Error(format!("Capture read error: {}", err)));
                    }
                }
            }
        }
    });

    Ok(CaptureWorker {
        stop,
        handle: Some(handle),
    })
}

#[cfg(target_os = "android")]
fn spawn_capture_worker(
    _iface_name: String,
    _local_ips: Vec<IpAddr>,
    _tx: mpsc::UnboundedSender<DashEvent>,
) -> Result<CaptureWorker, String> {
    Err("Packet capture is unavailable on Android in this build. System panels still work.".to_string())
}

#[cfg(not(target_os = "android"))]
fn parse_packet(packet: &[u8], local_ips: &HashSet<IpAddr>) -> Option<PacketEvent> {
    let eth = EthernetPacket::new(packet)?;
    if eth.get_ethertype() != EtherTypes::Ipv4 {
        return None;
    }

    let ipv4 = Ipv4Packet::new(eth.payload())?;
    let src_ip = IpAddr::V4(ipv4.get_source());
    let dst_ip = IpAddr::V4(ipv4.get_destination());
    let size = packet.len() as u64;

    match ipv4.get_next_level_protocol() {
        IpNextHeaderProtocols::Tcp => {
            let tcp = TcpPacket::new(ipv4.payload())?;
            let src_port = tcp.get_source();
            let dst_port = tcp.get_destination();
            let state = tcp_state(&tcp);
            build_event(
                "TCP",
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                state,
                size,
                local_ips,
            )
        }
        IpNextHeaderProtocols::Udp => {
            let udp = UdpPacket::new(ipv4.payload())?;
            build_event(
                "UDP",
                src_ip,
                dst_ip,
                udp.get_source(),
                udp.get_destination(),
                "ACTIVE".to_string(),
                size,
                local_ips,
            )
        }
        IpNextHeaderProtocols::Icmp => {
            let icmp = IcmpPacket::new(ipv4.payload())?;
            build_event(
                "ICMP",
                src_ip,
                dst_ip,
                0,
                0,
                format!("{:?}", icmp.get_icmp_type()),
                size,
                local_ips,
            )
        }
        _ => None,
    }
}

#[cfg(not(target_os = "android"))]
fn tcp_state(tcp: &TcpPacket) -> String {
    let f = tcp.get_flags();
    let syn = pnet::packet::tcp::TcpFlags::SYN;
    let ack = pnet::packet::tcp::TcpFlags::ACK;
    let fin = pnet::packet::tcp::TcpFlags::FIN;
    let rst = pnet::packet::tcp::TcpFlags::RST;
    let psh = pnet::packet::tcp::TcpFlags::PSH;

    if (f & syn) != 0 && (f & ack) != 0 {
        "SYN-ACK".to_string()
    } else if (f & syn) != 0 {
        "SYN".to_string()
    } else if (f & fin) != 0 {
        "FIN".to_string()
    } else if (f & rst) != 0 {
        "RST".to_string()
    } else if (f & psh) != 0 {
        "PSH".to_string()
    } else if (f & ack) != 0 {
        "ACK".to_string()
    } else {
        "DATA".to_string()
    }
}

fn build_event(
    proto: &str,
    src_ip: IpAddr,
    dst_ip: IpAddr,
    src_port: u16,
    dst_port: u16,
    state: String,
    size: u64,
    local_ips: &HashSet<IpAddr>,
) -> Option<PacketEvent> {
    let src = if src_port > 0 {
        format!("{}:{}", src_ip, src_port)
    } else {
        src_ip.to_string()
    };
    let dst = if dst_port > 0 {
        format!("{}:{}", dst_ip, dst_port)
    } else {
        dst_ip.to_string()
    };

    let (direction, remote_host, endpoint) = if local_ips.contains(&src_ip) && !local_ips.contains(&dst_ip)
    {
        (
            DirectionKind::Upload,
            dst_ip.to_string(),
            if dst_port > 0 {
                format!("{}:{}", dst_ip, dst_port)
            } else {
                dst_ip.to_string()
            },
        )
    } else if local_ips.contains(&dst_ip) && !local_ips.contains(&src_ip) {
        (
            DirectionKind::Download,
            src_ip.to_string(),
            if src_port > 0 {
                format!("{}:{}", src_ip, src_port)
            } else {
                src_ip.to_string()
            },
        )
    } else {
        (
            DirectionKind::Other,
            dst_ip.to_string(),
            if dst_port > 0 {
                format!("{}:{}", dst_ip, dst_port)
            } else {
                dst_ip.to_string()
            },
        )
    };

    Some(PacketEvent {
        proto: proto.to_string(),
        src,
        dst,
        size,
        state,
        endpoint,
        remote_host,
        direction,
    })
}

fn prune_hosts(hosts: &mut HashMap<String, u64>) {
    let mut pairs: Vec<_> = hosts.iter().map(|(k, v)| (k.clone(), *v)).collect();
    pairs.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
    pairs.truncate(200);
    hosts.clear();
    for (k, v) in pairs {
        hosts.insert(k, v);
    }
}

fn prune_connections(conns: &mut HashMap<String, ConnectionRow>) {
    let mut rows: Vec<_> = conns.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    rows.sort_by_key(|(_, v)| std::cmp::Reverse(v.bytes));
    rows.truncate(120);
    conns.clear();
    for (k, v) in rows {
        conns.insert(k, v);
    }
}

fn read_gpu_usage_percent() -> Option<f32> {
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

fn ui(f: &mut Frame, app: &mut App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_header(f, app, root[0]);

    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Min(8),
        ])
        .split(root[1]);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Min(20)])
        .split(body[0]);
    render_interfaces(f, app, top[0]);
    render_traffic_graph(f, app, top[1]);

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(36), Constraint::Min(20)])
        .split(body[1]);
    render_system_overview(f, app, mid[0]);
    render_top_processes(f, app, mid[1]);

    let bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(8)])
        .split(body[2]);
    render_packets(f, app, bottom[0]);

    let tail = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(bottom[1]);
    render_hosts(f, app, tail[0]);
    render_connections(f, app, tail[1]);

    render_footer(f, app, root[2]);

    if app.show_help {
        render_help(f);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let up = app.upload_hist.last().copied().unwrap_or(0);
    let down = app.download_hist.last().copied().unwrap_or(0);
    let gpu = app
        .gpu_usage
        .map(|v| format!("GPU {:.0}%", v))
        .unwrap_or_else(|| "GPU n/a".to_string());
    let title = format!(
        "NetCat++  {}  {}  CPU {:.0}%  MEM {:.1}/{:.1} GB  {}  ↑ {:.2} MB/s  ↓ {:.2} MB/s",
        app.selected_interface_name(),
        if app.paused { "[PAUSED]" } else { "[LIVE]" },
        app.cpu_usage,
        app.mem_used as f64 / 1024.0 / 1024.0 / 1024.0,
        app.mem_total as f64 / 1024.0 / 1024.0 / 1024.0,
        gpu,
        up as f64 / 1_000_000.0,
        down as f64 / 1_000_000.0
    );

    let block = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL).title("Dashboard"));
    f.render_widget(block, area);
}

fn render_interfaces(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .interfaces
        .iter()
        .map(|i| ListItem::new(format!("  {}", i.name)))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Interfaces"))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut app.interface_state);
}

fn render_traffic_graph(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let up = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title("Upload (last 60s)"))
        .data(&app.upload_hist)
        .style(Style::default().fg(Color::Green));
    f.render_widget(up, chunks[0]);

    let down = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title("Download (last 60s)"))
        .data(&app.download_hist)
        .style(Style::default().fg(Color::Blue));
    f.render_widget(down, chunks[1]);
}

fn render_system_overview(f: &mut Frame, app: &App, area: Rect) {
    let gpu = app
        .gpu_usage
        .map(|v| format!("{:.0}%", v))
        .unwrap_or_else(|| "n/a".to_string());
    let text = vec![
        Line::from(format!("CPU Usage       {:>6.2}%", app.cpu_usage)),
        Line::from(format!(
            "Memory Used     {:>6.2} GB",
            app.mem_used as f64 / 1024.0 / 1024.0 / 1024.0
        )),
        Line::from(format!(
            "Memory Total    {:>6.2} GB",
            app.mem_total as f64 / 1024.0 / 1024.0 / 1024.0
        )),
        Line::from(format!("iGPU (macOS)    {:>6}", gpu)),
        Line::from("Hint: j/k to switch interface"),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("System"))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn render_top_processes(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .top_processes
        .iter()
        .map(|p| {
            ListItem::new(format!(
                "{:>6} {:<18} {:>6.1}% {:>5}MB",
                p.pid,
                truncate(&p.name, 18),
                p.cpu,
                p.mem_mb
            ))
        })
        .collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Top Processes"));
    f.render_widget(list, area);
}

fn render_packets(f: &mut Frame, app: &App, area: Rect) {
    let filter = app.filter_input.value();
    let mut rows = Vec::new();
    for p in app.packets.iter().rev().take(18) {
        let line = format!(
            "{}  {:<4}  {:<21} -> {:<21} {:>5}B",
            p.time, p.proto, p.src, p.dst, p.bytes
        );
        if !filter.is_empty() && !line.contains(filter) {
            continue;
        }
        let color = match p.proto.as_str() {
            "TCP" => Color::Cyan,
            "UDP" => Color::Yellow,
            "ICMP" => Color::Green,
            _ => Color::White,
        };
        rows.push(ListItem::new(Line::styled(line, Style::default().fg(color))));
    }

    let list = List::new(rows).block(Block::default().borders(Borders::ALL).title("Live Packets"));
    f.render_widget(list, area);
}

fn render_hosts(f: &mut Frame, app: &App, area: Rect) {
    let mut hosts: Vec<_> = app.host_totals.iter().collect();
    hosts.sort_by_key(|(_, b)| std::cmp::Reverse(**b));

    let total: u64 = app.host_totals.values().sum();
    let mut items = Vec::new();
    for (host, bytes) in hosts.into_iter().take(6) {
        let pct = if total == 0 {
            0.0
        } else {
            (*bytes as f64 / total as f64) * 100.0
        };
        items.push(ListItem::new(format!("{:<20} {:>6.2}%", host, pct)));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Top Hosts by Traffic"));
    f.render_widget(list, area);
}

fn render_connections(f: &mut Frame, app: &App, area: Rect) {
    let mut rows: Vec<ConnectionRow> = app.connection_map.values().cloned().collect();
    if app.sorting_desc {
        rows.sort_by_key(|r| std::cmp::Reverse(r.bytes));
    } else {
        rows.sort_by_key(|r| r.bytes);
    }

    let items: Vec<ListItem> = rows
        .into_iter()
        .take(6)
        .map(|r| {
            ListItem::new(format!(
                "{:<4} {:<22} {:<12} {:>7}B",
                r.proto, r.endpoint, r.state, r.bytes
            ))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Active Connections"));
    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let mut status = format!(
        "Tab panel | f filter | s sort | p pause | ? help | q quit | filter='{}'",
        app.filter_input.value()
    );
    if let Some(err) = &app.last_error {
        status.push_str(&format!(" | error: {}", err));
    }

    let para = Paragraph::new(status)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(para, area);
}

fn render_help(f: &mut Frame) {
    let popup = centered_rect(60, 40, f.area());
    f.render_widget(Clear, popup);

    let text = vec![
        Line::from("Keyboard"),
        Line::from("Tab   cycle between panels"),
        Line::from("f     open filter input"),
        Line::from("s     sort active connections"),
        Line::from("p     pause/resume packet updates"),
        Line::from("j/k   switch active interface"),
        Line::from("q     quit"),
        Line::from("?     toggle this help"),
    ];

    let help = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: true });

    f.render_widget(help, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    s.chars().take(n.saturating_sub(3)).collect::<String>() + "..."
}
