pub mod api;
#[cfg(not(target_os = "android"))]
pub mod dashboard;
#[cfg(target_os = "android")]
#[path = "dashboard_android.rs"]
pub mod dashboard;
pub mod discovery;
pub mod http;
pub mod scanner;
pub mod sniffer;
pub mod transfer;
pub mod utils;

pub const NCP_CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
