// src/handlers/servers.rs
use actix_web::{web, HttpResponse, HttpRequest, Responder, error::ErrorTooManyRequests};
use capnp::message::Builder;
use log::{debug, error};
use crate::storage::memory::ServerStorage;
use crate::schema::server_list;
use governor::{RateLimiter, clock::DefaultClock};
use std::net::IpAddr;
use governor::state::keyed::DefaultKeyedStateStore;
use serde::Deserialize;
use crate::utils::{extract_real_ip, RequestError};


pub async fn get_servers(
    storage: web::Data<ServerStorage>,
    rate_limiter: web::Data<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>,
    req: HttpRequest,
) -> Result<HttpResponse, RequestError> {
    // Use the new extract real IP function
    let peer_ip = extract_real_ip(&req)?;

    // Rate Limiting
    if !rate_limiter.check_key(&peer_ip).is_ok() {
       error!("Rate limit exceeded for server list for ip: {}", peer_ip);
        return Err(RequestError::RateLimitExceeded);
    }

    storage.cleanup_stale_servers();
    let servers = storage.get_servers();

    debug!("Building server list response with {} servers", servers.len());

    let mut message = Builder::new_default();
    let server_list = message.init_root::<server_list::Builder>();
    let mut server_list_data = server_list.init_servers(servers.len() as u32);

    for (i, server) in servers.iter().enumerate() {
        let mut server_data = server_list_data.reborrow().get(i as u32);
        server_data.set_hostname(&server.host_name);
        server_data.set_map_name(&server.map_name);
        server_data.set_game_mode(&server.game_mode);
        server_data.set_max_players(server.max_players);
        server_data.set_port(server.port);
        server_data.set_ip(&server.ip);

        let mut player_list = server_data.init_players(server.players.len() as u32);
        for (j, player) in server.players.iter().enumerate() {
            let mut player_data = player_list.reborrow().get(j as u32);
            player_data.set_name(&player.name);
            player_data.set_gen(player.gen);
            player_data.set_lvl(player.lvl);
            player_data.set_team(player.team);
        }
    }

    let mut response_data = Vec::new();
    capnp::serialize::write_message(&mut response_data, &message)
        .expect("Failed to serialize server list");

    Ok(HttpResponse::Ok()
        .content_type("application/x-capnproto")
        .body(response_data))
}

#[derive(Deserialize)]
pub struct DeleteServerQuery {
    port: i32,
}

pub async fn delete_server(
    storage: web::Data<ServerStorage>,
    req: HttpRequest,
    query: web::Query<DeleteServerQuery>,
    rate_limiter: web::Data<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>,
) -> Result<HttpResponse, RequestError> {
    let peer_ip = extract_real_ip(&req)?;

    // Rate Limiting
    if !rate_limiter.check_key(&peer_ip).is_ok() {
        error!("Rate limit exceeded for server delete for ip: {}", peer_ip);
        return Err(RequestError::RateLimitExceeded);
    }

    let servers = storage.get_servers();
    let server_id = servers.iter()
        .find(|server| server.ip == peer_ip.to_string() && server.port == query.port)
        .map(|server| server.id.clone());

    match server_id {
        Some(id) => {
            storage.remove_server(&id);
            debug!("Removed server {}:{}", peer_ip, query.port);
            Ok(HttpResponse::Ok().finish())
        }
        None => {
            error!("Server not found for {}:{}", peer_ip, query.port);
           Ok(HttpResponse::NotFound().body("Server not found"))
        }
    }
}
