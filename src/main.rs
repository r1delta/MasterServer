// src/main.rs
mod config;
mod schema;
mod models;
mod handlers;
mod storage;
mod cloudflare;
mod utils;

use actix_web::{ web, App, HttpServer };
use env_logger::Env;
use storage::memory::ServerStorage;
use governor::{ RateLimiter, clock::DefaultClock };
use std::net::IpAddr;
use governor::state::keyed::DefaultKeyedStateStore;
use crate::config::Config;
use log::info;
use dotenv;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger only once at the start
    env_logger::init_from_env(Env::default().default_filter_or("debug"));

    // Initialize Cloudflare ranges
    if let Err(e) = cloudflare::initialize_cloudflare_ranges().await {
        log::error!("Failed to initialize Cloudflare IP ranges: {}", e);
        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to initialize Cloudflare ranges: {}", e)
            )
        );
    }

    // Load configuration
    let config = Config::from_env();

    dotenv::dotenv().ok();

    // Get bind address and port from environment or use defaults
    let bind_address = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "80".to_string());
    let bind = format!("{}:{}", bind_address, port);

    let storage = web::Data::new(ServerStorage::new(config.clone()));

    // Set up rate limiters using config
    let heartbeat_rate_limiter: web::Data<
        RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>
    > = web::Data::new(RateLimiter::keyed(config.heartbeat_quota()));

    let server_list_rate_limiter: web::Data<
        RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>
    > = web::Data::new(RateLimiter::keyed(config.server_list_quota()));

    let server_delete_rate_limiter: web::Data<
        RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>
    > = web::Data::new(RateLimiter::keyed(config.server_delete_quota()));

    info!("Starting server on {}", bind);
    HttpServer::new(move || {
        App::new()
            .app_data(storage.clone())
            .app_data(heartbeat_rate_limiter.clone())
            .app_data(server_list_rate_limiter.clone())
            .app_data(server_delete_rate_limiter.clone())
            .route("/auth", web::get().to(handlers::auth::handle_auth))
            .route("/server/heartbeat", web::post().to(handlers::heartbeat::handle_heartbeat))
            .route("/server/", web::get().to(handlers::servers::get_servers))
            .route("/server/delete", web::post().to(handlers::servers::delete_server))
    })
        .bind(&bind)?
        .run().await
}
