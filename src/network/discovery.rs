use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const DISCOVERY_PORT: u16 = 45454;
pub const DISCOVERY_REQUEST: &[u8] = b"DISCOVER_APOTEK_SERVER";
pub const DISCOVERY_RESPONSE: &[u8] = b"APOTEK_SERVER_OK|PORT:3306";

/// Menjalankan UDP listener untuk PC Gudang (Mode Server)
pub fn start_server_discovery_responder(running: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let addr = format!("0.0.0.0:{}", DISCOVERY_PORT);
        let socket = match UdpSocket::bind(&addr) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[DISCOVERY SERVER] Gagal bind socket UDP {}: {}", addr, e);
                return;
            }
        };

        let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));
        println!("[DISCOVERY SERVER] UDP Listener aktif di port {}", DISCOVERY_PORT);

        let mut buf = [0u8; 128];
        while running.load(Ordering::Relaxed) {
            match socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    let msg = &buf[..len];
                    if msg == DISCOVERY_REQUEST {
                        println!("[DISCOVERY SERVER] Menerima request dari {}, membalas...", src);
                        let _ = socket.send_to(DISCOVERY_RESPONSE, src);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Normal timeout check loop
                }
                Err(e) => {
                    eprintln!("[DISCOVERY SERVER] Error recv_from: {}", e);
                }
            }
        }
        println!("[DISCOVERY SERVER] Listener berhenti.");
    });
}

/// Mencari PC Gudang di jaringan LAN (Mode Client Kasir)
/// Mengembalikan Option<String> berupa IP Server (misal "192.168.1.100")
pub fn scan_server_ip(timeout_secs: u64) -> Option<String> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[DISCOVERY CLIENT] Gagal bind socket: {}", e);
            return None;
        }
    };

    let _ = socket.set_broadcast(true);
    let _ = socket.set_read_timeout(Some(Duration::from_secs(timeout_secs)));

    let target_addr: SocketAddr = format!("255.255.255.255:{}", DISCOVERY_PORT)
        .parse()
        .unwrap();

    // Kirim sinyal broadcast
    if let Err(e) = socket.send_to(DISCOVERY_REQUEST, target_addr) {
        eprintln!("[DISCOVERY CLIENT] Gagal broadcast: {}", e);
        return None;
    }

    let mut buf = [0u8; 128];
    match socket.recv_from(&mut buf) {
        Ok((len, src)) => {
            let resp = String::from_utf8_lossy(&buf[..len]);
            if resp.starts_with("APOTEK_SERVER_OK") {
                let ip = src.ip().to_string();
                println!("[DISCOVERY CLIENT] Server ditemukan di: {}", ip);
                return Some(ip);
            }
        }
        Err(e) => {
            eprintln!("[DISCOVERY CLIENT] Timeout atau server tidak merespons: {}", e);
        }
    }

    None
}
