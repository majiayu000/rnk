//! use_online hook for network connectivity detection
//!
//! Provides a way to detect if the system has network connectivity.
//!
//! # Example
//!
//! ```rust,ignore
//! use rnk::prelude::*;
//!
//! fn app() -> Element {
//!     let is_online = use_online();
//!
//!     if is_online {
//!         Text::new("Connected to network").into_element()
//!     } else {
//!         Text::new("Offline").into_element()
//!     }
//! }
//! ```

use crate::hooks::use_signal::use_signal;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Check if the system has network connectivity
///
/// Attempts to connect to common DNS servers to verify connectivity.
pub fn check_online() -> bool {
    // Try to connect to common DNS servers
    let targets = [
        SocketAddr::from(([8, 8, 8, 8], 53)),         // Google DNS
        SocketAddr::from(([1, 1, 1, 1], 53)),         // Cloudflare DNS
        SocketAddr::from(([208, 67, 222, 222], 53)), // OpenDNS
    ];

    for target in targets {
        if let Ok(stream) = TcpStream::connect_timeout(&target, Duration::from_millis(500)) {
            drop(stream);
            return true;
        }
    }

    false
}

/// Check if a specific host is reachable
pub fn check_host_reachable(host: &str, port: u16) -> bool {
    if let Ok(addr) = format!("{}:{}", host, port).parse() {
        if let Ok(stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(1000)) {
            drop(stream);
            return true;
        }
    }
    false
}

/// Hook to check if the system is online
///
/// Returns true if network connectivity is detected.
pub fn use_online() -> bool {
    let online = use_signal(check_online);
    online.get()
}

/// Hook to check if a specific host is reachable
pub fn use_host_reachable(host: &str, port: u16) -> bool {
    let host = host.to_string();
    let reachable = use_signal(move || check_host_reachable(&host, port));
    reachable.get()
}

/// Network status information
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NetworkStatus {
    /// Whether the system is online
    pub online: bool,
    /// Latency to DNS server in milliseconds (if online)
    pub latency_ms: Option<u32>,
}

impl NetworkStatus {
    /// Check network status with latency measurement
    pub fn check() -> Self {
        let start = std::time::Instant::now();
        let online = check_online();
        let latency_ms = if online {
            Some(start.elapsed().as_millis() as u32)
        } else {
            None
        };

        Self { online, latency_ms }
    }
}

/// Hook to get detailed network status
pub fn use_network_status() -> NetworkStatus {
    let status = use_signal(NetworkStatus::check);
    status.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_status_default() {
        let status = NetworkStatus::default();
        assert!(!status.online);
        assert!(status.latency_ms.is_none());
    }

    #[test]
    fn test_check_online_returns_bool() {
        // Just verify it returns without panicking
        let _ = check_online();
    }

    #[test]
    fn test_check_host_reachable_invalid() {
        // Invalid host should return false
        assert!(!check_host_reachable(
            "invalid.host.that.does.not.exist",
            80
        ));
    }

    #[test]
    fn test_network_status_check() {
        let status = NetworkStatus::check();
        // If online, latency should be Some
        if status.online {
            assert!(status.latency_ms.is_some());
        } else {
            assert!(status.latency_ms.is_none());
        }
    }

    #[test]
    fn test_use_online_compiles() {
        fn _test() {
            let _ = use_online();
        }
    }

    #[test]
    fn test_use_host_reachable_compiles() {
        fn _test() {
            let _ = use_host_reachable("example.com", 80);
        }
    }

    #[test]
    fn test_use_network_status_compiles() {
        fn _test() {
            let _ = use_network_status();
        }
    }
}
