use std::env;
use std::time::Duration;
use std::num::NonZeroU32;
use governor::Quota;

#[derive(Clone)]
pub struct Config {
    // Rate limiting configs
    pub heartbeat_period_secs: u64,
    pub heartbeat_burst_limit: u32,
    pub server_list_period_secs: u64,
    pub server_list_burst_limit: u32,
    pub server_delete_period_secs: u64,
    pub server_delete_burst_limit: u32,
    
    // Server limits
    pub max_servers_per_ip: usize,
    
    // Other configs
    pub server_timeout_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Default values matching your current configuration
            heartbeat_period_secs: 60,
            heartbeat_burst_limit: 100,
            server_list_period_secs: 5,
            server_list_burst_limit: 1,
            server_delete_period_secs: 5,
            server_delete_burst_limit: 1,
            max_servers_per_ip: 3,
            server_timeout_secs: 300, // 5 minutes
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            heartbeat_period_secs: env::var("HEARTBEAT_PERIOD_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
                
            heartbeat_burst_limit: env::var("HEARTBEAT_BURST_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
                
            server_list_period_secs: env::var("SERVER_LIST_PERIOD_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
                
            server_list_burst_limit: env::var("SERVER_LIST_BURST_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(120),
                
            server_delete_period_secs: env::var("SERVER_DELETE_PERIOD_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
                
            server_delete_burst_limit: env::var("SERVER_DELETE_BURST_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
                
            max_servers_per_ip: env::var("MAX_SERVERS_PER_IP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8),
                
            server_timeout_secs: env::var("SERVER_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }
    
    pub fn heartbeat_quota(&self) -> Quota {
        Quota::with_period(Duration::from_secs(self.heartbeat_period_secs))
            .unwrap()
            .allow_burst(NonZeroU32::new(self.heartbeat_burst_limit).unwrap())
    }
    
    pub fn server_list_quota(&self) -> Quota {
        Quota::with_period(Duration::from_secs(self.server_list_period_secs))
            .unwrap()
            .allow_burst(NonZeroU32::new(self.server_list_burst_limit).unwrap())
    }
    
    pub fn server_delete_quota(&self) -> Quota {
        Quota::with_period(Duration::from_secs(self.server_delete_period_secs))
            .unwrap()
            .allow_burst(NonZeroU32::new(self.server_delete_burst_limit).unwrap())
    }
}
