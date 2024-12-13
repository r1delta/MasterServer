// src/cloudflare.rs
use std::net::IpAddr;
use ipnetwork::IpNetwork;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use tokio::sync::OnceCell;
use std::str::FromStr;
use log::{info, error};

static CLOUDFLARE_RANGES: OnceCell<CloudflareRanges> = OnceCell::const_new();

lazy_static! {
    static ref IPV4_URL: &'static str = "https://www.cloudflare.com/ips-v4/";
    static ref IPV6_URL: &'static str = "https://www.cloudflare.com/ips-v6/";
}

#[derive(Debug, Default)]
pub struct CloudflareRanges {
    ipv4: RwLock<Vec<IpNetwork>>,
    ipv6: RwLock<Vec<IpNetwork>>
}

impl CloudflareRanges {
    pub fn new() -> Self {
        Self {
            ipv4: RwLock::new(Vec::new()),
            ipv6: RwLock::new(Vec::new())
        }
    }

    pub fn is_cloudflare_ip(&self, ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                let ranges = self.ipv4.read();
                ranges.iter().any(|network| network.contains(IpAddr::V4(ipv4)))
            },
            IpAddr::V6(ipv6) => {
                let ranges = self.ipv6.read();
                ranges.iter().any(|network| network.contains(IpAddr::V6(ipv6)))
            }
        }
    }
}

async fn fetch_ip_ranges(url: &str) -> Result<Vec<IpNetwork>, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(url).await?;
    let text = response.text().await?;
    
    let mut networks = Vec::new();
    for line in text.lines() {
        if let Ok(network) = IpNetwork::from_str(line.trim()) {
            networks.push(network);
        }
    }
    
    Ok(networks)
}

pub async fn initialize_cloudflare_ranges() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ranges = CloudflareRanges::new();
    
    info!("Fetching Cloudflare IPv4 ranges...");
    match fetch_ip_ranges(*IPV4_URL).await {
        Ok(networks) => {
            info!("Loaded {} IPv4 ranges", networks.len());
            *ranges.ipv4.write() = networks;
        },
        Err(e) => {
            error!("Failed to fetch IPv4 ranges: {}", e);
            return Err(e);
        }
    }
    
    info!("Fetching Cloudflare IPv6 ranges...");
    match fetch_ip_ranges(*IPV6_URL).await {
        Ok(networks) => {
            info!("Loaded {} IPv6 ranges", networks.len());
            *ranges.ipv6.write() = networks;
        },
        Err(e) => {
            error!("Failed to fetch IPv6 ranges: {}", e);
            return Err(e);
        }
    }
    
    CLOUDFLARE_RANGES.set(ranges).map_err(|_| "Failed to set Cloudflare ranges")?;
    Ok(())
}

pub fn verify_cloudflare_request(connecting_ip: IpAddr) -> bool {
    CLOUDFLARE_RANGES
        .get()
        .map(|ranges| ranges.is_cloudflare_ip(connecting_ip))
        .unwrap_or(false)
}
