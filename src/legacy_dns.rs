//! DNS fallback for the old Android ARM musl APK. Android 2.x keeps nameservers
//! in system properties instead of `/etc/resolv.conf`, which musl reads.

use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
    process::Command,
    time::Duration,
};

const GETPROP: &str = "/system/bin/getprop";

pub(crate) fn lookup(host: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Ok(vec![SocketAddr::V4(SocketAddrV4::new(ip, port))]);
    }
    let mut last_error = None;
    for key in ["net.dns1", "net.dns2"] {
        let output = match Command::new(GETPROP).arg(key).output() {
            Ok(output) => output,
            Err(error) => return Err(error),
        };
        let server = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let Ok(server) = server.parse::<Ipv4Addr>() else {
            continue;
        };
        match query(server, host, port) {
            Ok(addresses) if !addresses.is_empty() => return Ok(addresses),
            Ok(_) => {}
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error
        .unwrap_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Android DNS server not found")))
}

fn query(server: Ipv4Addr, host: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
    let mut packet = Vec::with_capacity(512);
    let id = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos()
        & 0xffff) as u16;
    packet.extend_from_slice(&id.to_be_bytes());
    packet.extend_from_slice(&[0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    for label in host.trim_end_matches('.').split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid DNS name",
            ));
        }
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.extend_from_slice(&[0, 0, 1, 0, 1]);

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    socket.connect(SocketAddrV4::new(server, 53))?;
    socket.send(&packet)?;
    let mut response = [0u8; 1500];
    let size = socket.recv(&mut response)?;
    let response = &response[..size];
    if response.len() < 12 || u16::from_be_bytes([response[0], response[1]]) != id {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid DNS response",
        ));
    }
    if response[3] & 0x0f != 0 {
        return Err(io::Error::new(io::ErrorKind::NotFound, "DNS lookup failed"));
    }
    let questions = u16::from_be_bytes([response[4], response[5]]) as usize;
    let answers = u16::from_be_bytes([response[6], response[7]]) as usize;
    let mut offset = 12;
    for _ in 0..questions {
        offset = skip_name(response, offset)?;
        offset = offset.checked_add(4).ok_or_else(invalid_dns)?;
        if offset > response.len() {
            return Err(invalid_dns());
        }
    }
    let mut result = Vec::new();
    for _ in 0..answers {
        offset = skip_name(response, offset)?;
        if offset + 10 > response.len() {
            return Err(invalid_dns());
        }
        let kind = u16::from_be_bytes([response[offset], response[offset + 1]]);
        let class = u16::from_be_bytes([response[offset + 2], response[offset + 3]]);
        let length = u16::from_be_bytes([response[offset + 8], response[offset + 9]]) as usize;
        offset += 10;
        if offset + length > response.len() {
            return Err(invalid_dns());
        }
        if kind == 1 && class == 1 && length == 4 {
            let ip = Ipv4Addr::new(
                response[offset],
                response[offset + 1],
                response[offset + 2],
                response[offset + 3],
            );
            result.push(SocketAddr::V4(SocketAddrV4::new(ip, port)));
        }
        offset += length;
    }
    Ok(result)
}

fn skip_name(packet: &[u8], mut offset: usize) -> io::Result<usize> {
    loop {
        let size = *packet.get(offset).ok_or_else(invalid_dns)?;
        offset += 1;
        if size & 0xc0 == 0xc0 {
            if offset >= packet.len() {
                return Err(invalid_dns());
            }
            return Ok(offset + 1);
        }
        if size & 0xc0 != 0 {
            return Err(invalid_dns());
        }
        if size == 0 {
            return Ok(offset);
        }
        offset = offset.checked_add(size as usize).ok_or_else(invalid_dns)?;
        if offset > packet.len() {
            return Err(invalid_dns());
        }
    }
}

fn invalid_dns() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "malformed DNS response")
}
