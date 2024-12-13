// src/handlers/servers.rs
use actix_web::{ web, HttpResponse, HttpRequest, Responder, error::ErrorTooManyRequests };
use capnp::message::Builder;
use log::{ debug, error };
use crate::storage::memory::ServerStorage;
use crate::schema::server_list;
use governor::{ RateLimiter, clock::DefaultClock };
use std::net::IpAddr;
use governor::state::keyed::DefaultKeyedStateStore;
use serde::Deserialize;
use crate::utils::{ extract_real_ip, RequestError };

pub async fn index(
    req: HttpRequest,
    storage: web::Data<ServerStorage>,
    bytes: web::Bytes,
    rate_limiter: web::Data<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>
) -> Result<HttpResponse, RequestError> {
    Ok(HttpResponse::Ok().content_type("application/json").body("{\"status\": \"ok\"}"))
}
