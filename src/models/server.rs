// src/models/server.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub gen: i32,
    pub lvl: i32,
    pub team: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub id: String,
    pub host_name: String,
    pub map_name: String,
    pub game_mode: String,
    pub players: Vec<Player>,
    pub max_players: i32,
    pub port: i32,
    pub ip: String,
    pub last_heartbeat: u64,
}
