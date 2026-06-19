use thiserror::Error;

uniffi::setup_scaffolding!();

#[derive(Debug, Error, uniffi::Error)]
pub enum NcpError {
    #[error("{message}")]
    Runtime { message: String },
}

fn map_err(err: impl std::fmt::Display) -> NcpError {
    NcpError::Runtime {
        message: err.to_string(),
    }
}

#[uniffi::export]
pub fn ncp_version() -> String {
    ncp_core::NCP_CORE_VERSION.to_string()
}

#[uniffi::export]
pub fn scan_host_json(
    host: String,
    range: String,
    timeout: String,
    udp: bool,
    concurrency: u32,
) -> Result<String, NcpError> {
    let result = ncp_core::api::scan_host_json(host, range, timeout, udp, concurrency as usize);
    Ok(result)
}

#[uniffi::export]
pub fn http_request_json(
    url: String,
    method: String,
    body: Option<String>,
    repeat: u32,
    follow: bool,
    max_redirects: u32,
) -> Result<String, NcpError> {
    let result = ncp_core::api::http_request_json(url, method, body, repeat as usize, follow, max_redirects as usize);
    Ok(result)
}

#[uniffi::export]
pub fn send_text(
    host: String,
    port: u16,
    message: String,
    repeat: u32,
    interval: String,
) -> Result<String, NcpError> {
    ncp_core::api::send_text(host, port, message, repeat as usize, interval);
    Ok("ok".to_string())
}

#[uniffi::export]
pub fn system_snapshot_json() -> Result<String, NcpError> {
    let snapshot = ncp_core::api::system_snapshot();
    serde_json::to_string(&snapshot).map_err(map_err)
}

#[uniffi::export]
pub fn start_packet_capture(interface: Option<String>, proto_filter: Option<String>) {
    ncp_core::api::start_capture(interface, proto_filter)
}

#[uniffi::export]
pub fn poll_packets_json(max_count: u32) -> Result<String, NcpError> {
    let packets = ncp_core::api::poll_packets(max_count as usize);
    serde_json::to_string(&packets).map_err(map_err)
}

#[uniffi::export]
pub fn stop_packet_capture() {
    ncp_core::api::stop_capture()
}

#[uniffi::export]
pub fn start_text_listener(port: u16, keep_alive: bool) {
    ncp_core::api::start_text_listener(port, keep_alive)
}

#[uniffi::export]
pub fn poll_text_messages_json(max_count: u32) -> Result<String, NcpError> {
    let messages = ncp_core::api::poll_text_messages(max_count as usize);
    serde_json::to_string(&messages).map_err(map_err)
}

#[uniffi::export]
pub fn stop_text_listener() {
    ncp_core::api::stop_text_listener()
}
