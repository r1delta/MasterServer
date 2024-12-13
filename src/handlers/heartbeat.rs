// src/handlers/heartbeat.rs
use actix_web::{web, HttpResponse, HttpRequest};
use capnp::message::ReaderOptions;
use log::{debug, error};
use std::net::{IpAddr, SocketAddr};
use crate::storage::memory::ServerStorage;
use crate::models::server::{ServerInfo, Player};
use crate::schema::server_heartbeat;
use governor::{RateLimiter, clock::DefaultClock};
use governor::state::keyed::DefaultKeyedStateStore;
use crate::utils::{extract_real_ip, format_address_for_challenge, RequestError, log_all_headers};
use tokio::net::UdpSocket;
use rand::Rng;
use std::fmt::Write;

pub async fn handle_heartbeat(
    req: HttpRequest,
    storage: web::Data<ServerStorage>,
    bytes: web::Bytes,
    rate_limiter: web::Data<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>
) -> Result<HttpResponse, RequestError> {
    // Log all headers for debugging
    log_all_headers(&req);

    // Get the real IP address
    let real_ip = extract_real_ip(&req)?;
    debug!("Original IP from headers: {}", real_ip);

    // No longer normalize. Let the socket addr logic take care of it
    let normalized_ip = real_ip;
    debug!("Normalized IP for processing: {}", normalized_ip);

    // Rate Limiting
    if !rate_limiter.check_key(&normalized_ip).is_ok() {
        error!("Rate limit exceeded for heartbeat for ip: {}", normalized_ip);
        return Err(RequestError::RateLimitExceeded);
    }

    debug!("Received heartbeat request with {} bytes", bytes.len());

    if bytes.is_empty() {
        error!("Received empty heartbeat request!");
        return Ok(HttpResponse::BadRequest().body("Empty request"));
    }

    let mut slice = bytes.as_ref();
    let reader = match capnp::serialize::read_message_from_flat_slice(
        &mut slice,
        ReaderOptions::new()
    ) {
        Ok(reader) => reader,
        Err(e) => {
            error!("Failed to read Cap'n Proto message: {}", e);
            return Ok(HttpResponse::BadRequest().body(format!("Invalid message format: {}", e)));
        }
    };

    let heartbeat = match reader.get_root::<server_heartbeat::Reader>() {
        Ok(heartbeat) => heartbeat,
        Err(e) => {
            error!("Failed to get heartbeat root: {}", e);
            return Ok(HttpResponse::BadRequest().body(format!("Invalid heartbeat data: {}", e)));
        }
    };

     let claimed_port = heartbeat.get_port();

    // Format address properly for challenge
    let socket_addr = match format_address_for_challenge(normalized_ip, claimed_port) {
        Ok(addr) => addr,
        Err(e) => {
            error!("Invalid server address: {}: {}", normalized_ip, e);
             return Ok(HttpResponse::BadRequest().body(format!("Invalid server address: {}", e)));
        }
    };

    if !verify_challenge(&socket_addr).await {
        error!("Challenge response failed from {}:{}", normalized_ip, claimed_port);
        return Ok(HttpResponse::BadRequest().body("Challenge response failed"));
    }

    let hostname = heartbeat.get_hostname().unwrap_or("").to_string();
    let map_name = heartbeat.get_map_name().unwrap_or("").to_string();
    let game_mode = heartbeat.get_game_mode().unwrap_or("").to_string();
    let max_players = heartbeat.get_max_players();
    let port = heartbeat.get_port();
    
    // Perform the data validation checks:
     if hostname.is_empty() {
       error!("Invalid hostname: Empty value");
       return Ok(HttpResponse::BadRequest().body("Invalid hostname: Must be at least 1 char."));
     }
    if hostname.len() > 64 {
        error!("Invalid hostname: Too long. {}", hostname);
        return Ok(HttpResponse::BadRequest().body("Invalid hostname: Too long (max 64 chars)."));
    }
    if map_name.is_empty() {
       error!("Invalid map_name: Empty value");
       return Ok(HttpResponse::BadRequest().body("Invalid map_name: Must be at least 1 char."));
    }
    if map_name.len() > 32 || !map_name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        error!("Invalid map_name: {}, must be <= 32 chars, only a-z and underscore", map_name);
        return Ok(HttpResponse::BadRequest().body("Invalid map_name: must be <= 32 chars, only a-z and underscore."));
    }
     if game_mode.is_empty() {
       error!("Invalid game_mode: Empty value");
       return Ok(HttpResponse::BadRequest().body("Invalid game_mode: Must be at least 1 char."));
    }
    if game_mode.len() > 32 || !game_mode.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
         error!("Invalid game_mode: {}, must be <= 32 chars, only a-z and underscore", game_mode);
        return Ok(HttpResponse::BadRequest().body("Invalid game_mode: must be <= 32 chars, only a-z and underscore."));
    }
    if max_players >= 20 {
        error!("Invalid max_players: {}, must be less than 20", max_players);
        return Ok(HttpResponse::BadRequest().body("Invalid max_players: must be less than 20."));
    }
    if port <= 1024 {
        error!("Invalid port: {}, must be higher than 1024", port);
        return Ok(HttpResponse::BadRequest().body("Invalid port: must be higher than 1024."));
    }

    let players = match heartbeat.get_players() {
        Ok(player_list) => {
            let mut players = Vec::new();
            for player in player_list.iter() {
                 let player_name = player.get_name().unwrap_or("").to_string();
                  if player_name.is_empty() {
                    error!("Invalid player name: Empty value");
                    return Ok(HttpResponse::BadRequest().body("Invalid player name: Must be at least 1 char."));
                 }
                players.push(Player {
                    name: player_name,
                    gen: player.get_gen(),
                    lvl: player.get_lvl(),
                    team: player.get_team(),
                });
            }
            players
        },
        Err(e) => {
            error!("Failed to read players: {}", e);
            Vec::new()
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let server_info = ServerInfo {
        id: uuid::Uuid::new_v4().to_string(),
        host_name: hostname,
        map_name,
        game_mode,
        players,
        max_players,
        port,
        ip: normalized_ip.to_string(), // Store the normalized IP
        last_heartbeat: now,
    };

    match storage.add_server(server_info) {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => {
            error!("Failed to add server: {}", e);
            Ok(HttpResponse::BadRequest().body(e))
        }
    }
}

async fn verify_challenge(server_addr: &SocketAddr) -> bool {
    let mut rng = rand::thread_rng();
    let nonce_bytes: [u8; 4] = rng.gen();
    let mut nonce_str = String::from("0x");
    for byte in nonce_bytes {
        write!(&mut nonce_str, "{:02X}", byte).unwrap();
    }

    let mut challenge_packet: Vec<u8> = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x48];
    challenge_packet.extend_from_slice(b"connect");
    challenge_packet.extend_from_slice(nonce_str.as_bytes());
    challenge_packet.push(0x00);

    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(socket) => socket,
        Err(e) => {
            error!("Could not bind udp socket: {}", e);
            return false;
        }
    };

    match socket.send_to(&challenge_packet, server_addr).await {
        Ok(_) => debug!("Challenge sent to {} with nonce {}", server_addr, nonce_str),
        Err(e) => {
            error!("Error sending challenge to {}: {}", server_addr, e);
            return false;
        }
    };

    let mut buffer = [0u8; 1024];
    match tokio::time::timeout(std::time::Duration::from_secs(2), async {
        socket.recv_from(&mut buffer).await
    }).await {
        Ok(Ok((len, _addr))) => {
            if len < 21 || buffer[0] != 0xFF || buffer[1] != 0xFF || buffer[2] != 0xFF || buffer[3] != 0xFF || buffer[4] != 0x49 {
                error!("Received invalid challenge response from {} with len {}", server_addr, len);
                return false;
            }

            let response_nonce_str = String::from_utf8_lossy(&buffer[16..26]);
            if !response_nonce_str.starts_with("0x") {
                error!("Received invalid nonce format from {}, received: {}", server_addr, response_nonce_str);
                return false;
            }
            if response_nonce_str != nonce_str {
                error!("Received invalid nonce from {}, sent: {}, received: {}", 
                    server_addr, nonce_str, response_nonce_str);
                return false;
            }
             if String::from_utf8_lossy(&buffer[9..16]) != "connect" {
                error!("Received invalid connect string from {}, received: {:?}", 
                    server_addr, &buffer[9..16]);
                return false;
            }
            debug!("Received valid challenge response from {} with nonce: {}", server_addr, nonce_str);
            true
        },
        Ok(Err(e)) => {
            error!("Failed to receive challenge response from {}: {}", server_addr, e);
            false
        },
        Err(e) => {
            error!("Timed out receiving challenge response from {}: {}", server_addr, e);
            false
        }
    }
}
