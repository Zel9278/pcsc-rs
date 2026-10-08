use std::env;

pub const DEFAULT_URI: &str = "https://pcss.eov2.com";

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
    /// Server URL (`PCSC_URI`).
    pub uri: String,
    /// Password shared with the server (`PASS`).
    pub pass: String,
    /// Hostname shown on PC Status (`HOSTNAME`), the system's own when unset.
    pub hostname: Option<String>,
    /// Behaviour after a self-update (`PCSC_UPDATED`).
    pub on_update: OnUpdate,
}

impl Config {
    /// Reads the configuration from the environment. `None` when `PASS` is missing.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let pass = env::var("PASS").ok()?;
        Some(Self {
            uri: env::var("PCSC_URI").unwrap_or_else(|_| DEFAULT_URI.into()),
            pass,
            hostname: env::var("HOSTNAME").ok().filter(|h| !h.is_empty()),
            on_update: OnUpdate::parse(env::var("PCSC_UPDATED").ok().as_deref()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::OnUpdate;

    #[test]
    fn on_update_values() {
        assert_eq!(OnUpdate::parse(Some("restart")), OnUpdate::Restart);
        assert_eq!(OnUpdate::parse(Some("terminate")), OnUpdate::Terminate);
        assert_eq!(OnUpdate::parse(Some("none")), OnUpdate::Nothing);
        assert_eq!(OnUpdate::parse(Some("anything")), OnUpdate::Nothing);
        assert_eq!(OnUpdate::parse(None), OnUpdate::Nothing);
    }
}
