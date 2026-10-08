use std::env;

pub const DEFAULT_URI: &str = "wss://pcss.eov2.com/server";

/// What to do after the binary has replaced itself with a newer release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnUpdate {
    /// Keep running the old version; the new one is used from the next start.
    Nothing,
    /// Start the new binary and exit.
    Restart,
    /// Exit, leaving the restart to a service manager (systemd and the like).
    Terminate,
}

impl OnUpdate {
    fn parse(value: Option<&str>) -> Self {
        match value {
            Some("restart") => Self::Restart,
            Some("terminate") => Self::Terminate,
            _ => Self::Nothing,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    /// WebSocket URL of the server (from `PCSC_URI`).
    pub uri: String,
    /// Password shared with the server (`PASS`).
    pub pass: String,
    /// Hostname shown on PC Status (`HOSTNAME`), the system's own when unset.
    pub hostname: Option<String>,
    /// Lets several clients share a hostname (`DEV_MODE=true`); the server
    /// shows them as `[DEV] <hostname>_<n>`.
    pub dev: bool,
    /// Behaviour after a self-update (`PCSC_UPDATED`).
    pub on_update: OnUpdate,
}

impl Config {
    /// Reads the configuration from the environment. `None` when `PASS` is missing.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let pass = env::var("PASS").ok()?;
        Some(Self {
            uri: websocket_url(&env::var("PCSC_URI").unwrap_or_else(|_| DEFAULT_URI.into())),
            pass,
            hostname: env::var("HOSTNAME").ok().filter(|h| !h.is_empty()),
            dev: env::var("DEV_MODE").is_ok_and(|v| v.eq_ignore_ascii_case("true")),
            on_update: OnUpdate::parse(env::var("PCSC_UPDATED").ok().as_deref()),
        })
    }
}

/// v1 took the server's origin (`https://pcss.eov2.com`); v2 talks WebSocket
/// on `/server`. Accept both so existing `PCSC_URI` settings keep working.
fn websocket_url(uri: &str) -> String {
    let uri = uri.trim().trim_end_matches('/');
    let (scheme, rest) = match uri.split_once("://") {
        Some(("http" | "ws", rest)) => ("ws", rest),
        // https, wss, or anything unknown
        Some((_, rest)) => ("wss", rest),
        None => ("wss", uri),
    };
    if rest.contains('/') {
        format!("{scheme}://{rest}")
    } else {
        format!("{scheme}://{rest}/server")
    }
}

#[cfg(test)]
mod tests {
    use super::{OnUpdate, websocket_url};

    #[test]
    fn on_update_values() {
        assert_eq!(OnUpdate::parse(Some("restart")), OnUpdate::Restart);
        assert_eq!(OnUpdate::parse(Some("terminate")), OnUpdate::Terminate);
        assert_eq!(OnUpdate::parse(Some("none")), OnUpdate::Nothing);
        assert_eq!(OnUpdate::parse(Some("anything")), OnUpdate::Nothing);
        assert_eq!(OnUpdate::parse(None), OnUpdate::Nothing);
    }

    #[test]
    fn websocket_urls() {
        assert_eq!(
            websocket_url("https://pcss.eov2.com"),
            "wss://pcss.eov2.com/server"
        );
        assert_eq!(
            websocket_url("https://pcss.eov2.com/"),
            "wss://pcss.eov2.com/server"
        );
        assert_eq!(
            websocket_url("http://127.0.0.1:3000"),
            "ws://127.0.0.1:3000/server"
        );
        assert_eq!(
            websocket_url("wss://pcss.eov2.com/server"),
            "wss://pcss.eov2.com/server"
        );
        assert_eq!(
            websocket_url("ws://localhost:3000/ws"),
            "ws://localhost:3000/ws"
        );
        assert_eq!(websocket_url("pcss.eov2.com"), "wss://pcss.eov2.com/server");
    }
}
