// src/storage/memory.rs
use dashmap::DashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models::server::ServerInfo;
use crate::config::Config;

pub struct ServerStorage {
    servers: DashMap<String, ServerInfo>,
    config: Config,
}

impl ServerStorage {
    pub fn new(config: Config) -> Self {
        Self {
            servers: DashMap::new(),
            config,
        }
    }

    pub fn add_server(&self, server_info: ServerInfo) -> Result<(), String> {
        // Check if a server with the same IP and port already exists.
        let existing_server_id = self.servers
            .iter()
            .find(|r| r.value().ip == server_info.ip && r.value().port == server_info.port)
            .map(|r| r.key().clone());

        if let Some(id) = existing_server_id {
           self.servers.remove(&id);
        } else {
            // Check number of servers from this IP
            let server_count = self.servers
                .iter()
                .filter(|r| r.value().ip == server_info.ip)
                .count();
            
            if server_count >= self.config.max_servers_per_ip {
                return Err(format!("Maximum number of servers ({}) reached for this IP", self.config.max_servers_per_ip));
            }
        }
        
        self.servers.insert(server_info.id.clone(), server_info);
        Ok(())
    }

    pub fn cleanup_stale_servers(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        self.servers.retain(|_, server| {
            now - server.last_heartbeat < self.config.server_timeout_secs
        });
    }

    pub fn get_servers(&self) -> Vec<ServerInfo> {
        self.servers.iter().map(|r| r.value().clone()).collect()
    }

    pub fn remove_server(&self, id: &str) {
        self.servers.remove(id);
    }
}
