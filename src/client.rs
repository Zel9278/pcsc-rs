//! The WebSocket connection to the PC Status server (`/server`). Messages are
//! JSON `{"type", "data"}` objects, as in pc-status-monorepo-rs.
//!
//! The server says `Hi` when we connect; we answer `Hi` with the status and the
//! password. After that it sends `Sync` every second and we answer `Sync` with
//! the latest status. A `Close` from the server means it refused us (a wrong
//! password, or a client with the same hostname is already connected).

use std::{net::TcpStream, thread, time::Duration};

use serde::{Deserialize, Serialize};
use tungstenite::{Message, WebSocket, stream::MaybeTlsStream};

use crate::{config::Config, monitor::SharedStatus, status::SystemStatus};

const RETRY_MIN: Duration = Duration::from_secs(5);
const RETRY_MAX: Duration = Duration::from_secs(60);
/// The server sends `Sync` every second; this much silence means the connection is gone.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "data")]
enum ClientMessage<'a> {
    Hi {
        data: &'a SystemStatus,
        pass: &'a str,
    },
    Sync(&'a SystemStatus),
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
enum ServerMessage {
    Hi,
    Sync,
    Close,
    /// Anything else (`Status`, `Toast`, …) is meant for viewers.
    #[serde(other)]
    Other,
}

/// Only `type` matters to us; `data` is a greeting or a viewer payload.
#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "type")]
    kind: ServerMessage,
}

fn parse(text: &str) -> serde_json::Result<ServerMessage> {
    serde_json::from_str::<Envelope>(text).map(|e| e.kind)
}

/// How a connection ended.
enum End {
    /// The server refused this client.
    Refused,
    /// The connection dropped; `true` when it had been working.
    Lost(bool),
}

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

/// Keeps a connection to the server, reconnecting with a growing delay. Never returns.
pub fn run(config: &Config, status: &SharedStatus) -> ! {
    let mut wait = RETRY_MIN;
    loop {
        match session(config, status) {
            Ok(End::Refused) => {
                eprintln!(
                    "The server refused this client (wrong PASS, or the same hostname is already connected)"
                );
            }
            Ok(End::Lost(true)) => {
                println!("Disconnected");
                wait = RETRY_MIN;
            }
            Ok(End::Lost(false)) => println!("Disconnected"),
            Err(e) => eprintln!("Connection failed: {e}"),
        }
        eprintln!("Reconnecting in {}s", wait.as_secs());
        thread::sleep(wait);
        wait = (wait * 2).min(RETRY_MAX);
    }
}

fn session(config: &Config, status: &SharedStatus) -> tungstenite::Result<End> {
    let (mut socket, _) = tungstenite::connect(config.uri.as_str())?;
    set_read_timeout(&socket)?;
    println!("Connected");

    let mut registered = false;
    loop {
        let message = match socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                return Ok(End::Lost(registered));
            }
            Err(e) => {
                eprintln!("Error: {e}");
                return Ok(End::Lost(registered));
            }
        };
        let text = match message {
            Message::Text(text) => text,
            Message::Close(_) => return Ok(End::Lost(registered)),
            _ => continue,
        };
        match parse(&text) {
            Ok(ServerMessage::Hi) => {
                println!("Received hi");
                let current = status.load();
                send(
                    &mut socket,
                    &ClientMessage::Hi {
                        data: &current,
                        pass: &config.pass,
                    },
                )?;
            }
            Ok(ServerMessage::Sync) => {
                registered = true;
                send(&mut socket, &ClientMessage::Sync(&status.load()))?;
            }
            Ok(ServerMessage::Close) => {
                // The server closes the socket right after; let it finish.
                let _ = socket.close(None);
                return Ok(End::Refused);
            }
            Ok(ServerMessage::Other) => {}
            Err(e) => eprintln!("Unknown message ({e}): {text}"),
        }
    }
}

fn send(socket: &mut Socket, message: &ClientMessage) -> tungstenite::Result<()> {
    let json = serde_json::to_string(message).expect("the status always serializes");
    socket.send(Message::text(json))
}

fn set_read_timeout(socket: &Socket) -> std::io::Result<()> {
    let stream = match socket.get_ref() {
        MaybeTlsStream::Plain(stream) => stream,
        MaybeTlsStream::NativeTls(stream) => stream.get_ref(),
        _ => return Ok(()),
    };
    stream.set_read_timeout(Some(READ_TIMEOUT))
}

#[cfg(test)]
mod tests {
    use super::{ClientMessage, ServerMessage, parse};
    use crate::status;

    #[test]
    fn client_messages() {
        let status = status::sample();
        let hi = serde_json::to_value(ClientMessage::Hi {
            data: &status,
            pass: "secret",
        })
        .unwrap();
        assert_eq!(hi["type"], "Hi");
        assert_eq!(hi["data"]["pass"], "secret");
        assert_eq!(hi["data"]["data"]["hostname"], "host");

        let sync = serde_json::to_value(ClientMessage::Sync(&status)).unwrap();
        assert_eq!(sync["type"], "Sync");
        assert_eq!(sync["data"]["hostname"], "host");
    }

    #[test]
    fn server_messages() {
        let parse = |s: &str| parse(s).unwrap();
        assert_eq!(parse(r#"{"type":"Hi","data":"hello"}"#), ServerMessage::Hi);
        assert_eq!(
            parse(r#"{"type":"Sync","data":"sync"}"#),
            ServerMessage::Sync
        );
        assert_eq!(parse(r#"{"type":"Close"}"#), ServerMessage::Close);
        assert_eq!(
            parse(r#"{"type":"Status","data":{}}"#),
            ServerMessage::Other
        );
        assert_eq!(
            parse(r#"{"type":"Toast","data":{"message":"m","color":"c","toast_time":1}}"#),
            ServerMessage::Other
        );
    }
}
