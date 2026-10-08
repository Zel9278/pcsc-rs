//! The socket.io connection to the PC Status server (namespace `/server`).
//!
//! The server says `hi` when we connect; we answer `hi` with the status and the
//! password. After that it sends `sync` every second and we answer `sync` with
//! the latest status. A `close` from the server means it refused us (for
//! instance a client with the same hostname is already connected).

use std::{thread, time::Duration};

use rust_socketio::{
    Event, Payload, RawClient,
    client::{Client, ClientBuilder},
};
use serde_json::json;

use crate::{config::Config, monitor::SharedStatus};

const RETRY_MIN_MS: u64 = 5_000;
const RETRY_MAX_MS: u64 = 60_000;

/// Connects, retrying until the server is reachable. Once connected the
/// client reconnects on its own.
pub fn connect(config: &Config, status: &SharedStatus) -> Client {
    let mut wait = Duration::from_millis(RETRY_MIN_MS);
    loop {
        match builder(config, status.clone()).connect() {
            Ok(client) => return client,
            Err(e) => {
                eprintln!("Connection failed: {e}; retrying in {}s", wait.as_secs());
                thread::sleep(wait);
                wait = (wait * 2).min(Duration::from_millis(RETRY_MAX_MS));
            }
        }
    }
}

fn builder(config: &Config, status: SharedStatus) -> ClientBuilder {
    let pass = config.pass.clone();
    let hi_status = status.clone();

    ClientBuilder::new(config.uri.clone())
        .namespace("/server")
        .reconnect(true)
        .reconnect_on_disconnect(true)
        .reconnect_delay(RETRY_MIN_MS, RETRY_MAX_MS)
        .on(Event::Connect, |_, _| println!("Connected"))
        .on(Event::Close, |_, _| println!("Disconnected"))
        .on(Event::Error, |err, _| {
            eprintln!("Error: {}", describe(&err));
        })
        .on("hi", move |payload, socket: RawClient| {
            println!("Received hi: {}", describe(&payload));
            let current = hi_status.load();
            if let Err(e) = socket.emit("hi", json!(current.with_pass(&pass))) {
                eprintln!("Failed to send hi: {e}");
            }
        })
        .on("sync", move |_, socket: RawClient| {
            let current = status.load();
            if let Err(e) = socket.emit("sync", json!(current.as_ref())) {
                eprintln!("Failed to send sync: {e}");
            }
        })
        .on("close", |_, _| {
            eprintln!("The server refused this client (is the same hostname already connected?)");
        })
}

fn describe(payload: &Payload) -> String {
    match payload {
        Payload::Text(values) => values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", "),
        Payload::Binary(bytes) => format!("{} bytes", bytes.len()),
        #[allow(deprecated)]
        Payload::String(text) => text.clone(),
    }
}
