//! Monolog-like channel names as tracing `target` values.

/// Application / domain channel.
pub const APP: &str = "serenade::app";

/// HTTP request lifecycle channel.
pub const REQUEST: &str = "serenade::request";

/// Security / firewall channel.
pub const SECURITY: &str = "serenade::security";

/// Messenger / bus channel.
pub const MESSENGER: &str = "serenade::messenger";

/// Kernel / boot channel.
pub const KERNEL: &str = "serenade::kernel";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_targets_use_serenade_prefix() {
        for name in [APP, REQUEST, SECURITY, MESSENGER, KERNEL] {
            assert!(name.starts_with("serenade::"));
        }
    }
}
