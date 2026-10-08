//! Minimal browser native-messaging adapter. It only forwards validated HTTPS origins
//! to Timebridge's fixed IPv4 loopback endpoint; it exposes no shell or file API.
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{Read, Write};

const MAX_BODY: usize = 32768;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Message {
    origin: String,
    credential: Option<String>,
    request: Value,
}

fn validate_origin(origin: &str) -> Result<(), String> {
    if origin.len() > 512 { return Err("Origin too long".into()); }
    let url = url::Url::parse(origin).map_err(|_| "Invalid origin")?;
    let loopback = url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
    if (url.scheme() != "https" && !loopback) || !url.username().is_empty() || url.password().is_some()
        || url.origin().ascii_serialization() != origin {
        return Err("Use an exact HTTPS origin (HTTP allowed only for localhost development)".into());
    }
    Ok(())
}

fn exchange(input: &[u8]) -> Result<Value, String> {
    let message: Message = serde_json::from_slice(input).map_err(|_| "Invalid native message")?;
    validate_origin(&message.origin)?;
    if message.credential.as_ref().is_some_and(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit())) {
        return Err("Invalid credential".into());
    }
    let body = serde_json::to_vec(&message.request).map_err(|e| e.to_string())?;
    if body.len() > MAX_BODY { return Err("Request too large".into()); }
    let address = "127.0.0.1:47832".parse().unwrap();
    let mut socket = std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(2))
        .map_err(|_| "Open Timebridge Desktop first")?;
    socket.set_read_timeout(Some(std::time::Duration::from_secs(6))).map_err(|e| e.to_string())?;
    socket.set_write_timeout(Some(std::time::Duration::from_secs(2))).map_err(|e| e.to_string())?;
    let auth = message.credential.map(|s| format!("Authorization: Bearer {s}\r\n")).unwrap_or_default();
    let header = format!("POST /v1 HTTP/1.1\r\nHost: 127.0.0.1:47832\r\nOrigin: {}\r\nContent-Type: application/json\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n", message.origin, body.len());
    socket.write_all(header.as_bytes()).and_then(|_| socket.write_all(&body)).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    socket.take(1_048_577).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > 1_048_576 { return Err("Response too large".into()); }
    let start = bytes.windows(4).position(|s| s == b"\r\n\r\n").ok_or("Invalid bridge response")? + 4;
    serde_json::from_slice(&bytes[start..]).map_err(|_| "Invalid bridge JSON response".into())
}

fn main() {
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    loop {
        let mut length = [0u8; 4];
        if input.read_exact(&mut length).is_err() { break; }
        let length = u32::from_le_bytes(length) as usize;
        if length == 0 || length > MAX_BODY { break; }
        let mut body = vec![0; length];
        if input.read_exact(&mut body).is_err() { break; }
        let response = exchange(&body).unwrap_or_else(|e| json!({"error":e}));
        let bytes = serde_json::to_vec(&response).unwrap();
        if output.write_all(&(bytes.len() as u32).to_le_bytes()).and_then(|_| output.write_all(&bytes)).and_then(|_| output.flush()).is_err() { break; }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_origin;
    #[test]
    fn validates_exact_secure_origins_and_local_development_only() {
        assert!(validate_origin("https://example.com").is_ok());
        assert!(validate_origin("https://sub.example.com:8443").is_ok());
        assert!(validate_origin("http://localhost:5173").is_ok());
        assert!(validate_origin("http://127.0.0.1:3000").is_ok());
        assert!(validate_origin("https://example.com/path").is_err());
        assert!(validate_origin("http://192.168.1.5").is_err());
        assert!(validate_origin("https://user@example.com").is_err());
    }
}
