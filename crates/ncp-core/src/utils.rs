use dns_lookup::lookup_addr;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

static DNS_CACHE: Lazy<Mutex<HashMap<IpAddr, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub default_interface: Option<String>,
    pub default_scan_range: String,
    pub default_timeout: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_interface: None,
            default_scan_range: "1-1000".to_string(),
            default_timeout: "500ms".to_string(),
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    let base = dirs::config_dir()?;
    Some(base.join("ncp").join("config.json"))
}

pub fn load_or_init_config() -> AppConfig {
    let Some(path) = config_path() else {
        return AppConfig::default();
    };

    if let Ok(bytes) = fs::read(&path) {
        if let Ok(cfg) = serde_json::from_slice::<AppConfig>(&bytes) {
            return cfg;
        }
    }

    let cfg = AppConfig::default();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &path,
        serde_json::to_vec_pretty(&cfg).unwrap_or_else(|_| b"{}".to_vec()),
    );
    cfg
}

pub fn format_size(bytes: u64) -> String {
    let kb = 1024_u64;
    let mb = kb * 1024;
    let gb = mb * 1024;

    if bytes >= gb {
        format!("{:.2} GB", bytes as f64 / gb as f64)
    } else if bytes >= mb {
        format!("{:.2} MB", bytes as f64 / mb as f64)
    } else if bytes >= kb {
        format!("{:.2} KB", bytes as f64 / kb as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn parse_timeout_millis(s: &str) -> Result<u64, String> {
    if let Some(ms) = s.strip_suffix("ms") {
        return ms
            .trim()
            .parse::<u64>()
            .map_err(|_| format!("Invalid timeout: {s}"));
    }
    if let Some(sec) = s.strip_suffix('s') {
        return sec
            .trim()
            .parse::<u64>()
            .map(|v| v * 1000)
            .map_err(|_| format!("Invalid timeout: {s}"));
    }

    s.parse::<u64>()
        .map_err(|_| format!("Invalid timeout: {s}"))
}

pub fn parse_timeout_duration(s: &str) -> Result<Duration, String> {
    parse_timeout_millis(s).map(Duration::from_millis)
}

pub async fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 64 * 1024];

    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}

pub fn sha256_file_sync(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 64 * 1024];

    loop {
        let n = std::io::Read::read(&mut file, &mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn resolve_hostname_cached(ip: IpAddr) -> String {
    if let Ok(cache) = DNS_CACHE.lock() {
        if let Some(found) = cache.get(&ip) {
            return found.clone();
        }
    }

    let resolved = lookup_addr(&ip).unwrap_or_else(|_| ip.to_string());
    if let Ok(mut cache) = DNS_CACHE.lock() {
        cache.insert(ip, resolved.clone());
    }
    resolved
}

pub fn derive_key(passphrase: &str) -> [u8; 32] {
    let digest = Sha256::digest(passphrase.as_bytes());
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest);
    key
}

pub fn xor_keystream_in_place(buf: &mut [u8], key: &[u8; 32], counter: &mut u64) {
    if buf.is_empty() {
        return;
    }

    for chunk in buf.chunks_mut(32) {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(counter.to_be_bytes());
        let block = hasher.finalize();
        for (i, b) in chunk.iter_mut().enumerate() {
            *b ^= block[i];
        }
        *counter += 1;
    }
}
