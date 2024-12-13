// src/utils.rs
use actix_web::{HttpRequest, HttpResponse, ResponseError};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use log::error;
use log::debug;
use log::warn;
use crate::cloudflare::verify_cloudflare_request;
use std::fmt;

#[derive(Debug)]
pub enum RequestError {
    MissingPeerIP,
    NonCloudflareIP(String),
    MissingCFHeader,
    InvalidCFHeader,
    InvalidIPFormat,
    RateLimitExceeded,
    IPv6NotSupported,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPeerIP => write!(f, "Failed to extract client IP"),
            Self::NonCloudflareIP(ip) => write!(f, "Request from non-Cloudflare IP: {}", ip),
            Self::MissingCFHeader => write!(f, "Missing CF-Connecting-IP header"),
            Self::InvalidCFHeader => write!(f, "Invalid CF-Connecting-IP header"),
            Self::InvalidIPFormat => write!(f, "Invalid CF-Connecting-IP format"),
            Self::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            Self::IPv6NotSupported => write!(f, "IPv6 addresses are not supported"),
        }
    }
}

impl ResponseError for RequestError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::NonCloudflareIP(_) => {
                HttpResponse::Forbidden().body(self.to_string())
            }
             Self::RateLimitExceeded => {
                HttpResponse::TooManyRequests().body(self.to_string())
            }
            Self::IPv6NotSupported => {
                HttpResponse::BadRequest().body(self.to_string())
            }
            _ => HttpResponse::BadRequest().body(self.to_string())
        }
    }
}

pub fn extract_real_ip(req: &HttpRequest) -> Result<IpAddr, RequestError> {
    // Get the peer address FIRST. This is the IP of the connection, and we must validate it against
    // Cloudflare.
     let peer_addr = match req.peer_addr() {
        Some(addr) => addr.ip(),
        None => return Err(RequestError::MissingPeerIP)
     };
     if let IpAddr::V4(peer_v4) = peer_addr {
        if !verify_cloudflare_request(IpAddr::V4(peer_v4)) {
           return Err(RequestError::NonCloudflareIP(peer_v4.to_string()));
         }
     } else {
          return Err(RequestError::IPv6NotSupported);
      }
      
    // We are good, now we try to parse the forwarded headers.
    // Check X-Forwarded-For first
    if let Some(forwarded_for) = req.headers().get("X-Forwarded-For") {
        if let Ok(ip_str) = forwarded_for.to_str() {
            if let Some(first_ip) = ip_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                   if let IpAddr::V4(v4) = ip {
                       return Ok(IpAddr::V4(v4));
                    } else {
                        return Err(RequestError::IPv6NotSupported);
                    }
                }
            }
        }
    }
    
    // Check CF-Connecting-IP next
     if let Some(cf_ip) = req.headers().get("CF-Connecting-IP") {
       if let Ok(ip_str) = cf_ip.to_str() {
          if let Ok(ip) = ip_str.parse::<IpAddr>() {
            // Also check if client provided their real IPv4
            if let Some(true_ip) = req.headers().get("X-Real-IP") {
               if let Ok(true_ip_str) = true_ip.to_str() {
                  if let Ok(true_ip_addr) = true_ip_str.parse::<IpAddr>() {
                      if let IpAddr::V4(v4) = true_ip_addr {
                            debug!("Using provided X-Real-IP: {}", v4);
                            return Ok(IpAddr::V4(v4));
                      }
                   }
                }
            }
             if let IpAddr::V4(v4) = ip {
                 return Ok(IpAddr::V4(v4));
               } else {
                 return Err(RequestError::IPv6NotSupported);
                }
             }
         }
     }
    
    // If we get here, there's no forwarded IP. This is a problem, because we already validated the Cloudflare ip.
    Err(RequestError::MissingCFHeader)
}

// For debugging purposes, add this function
pub fn log_all_headers(req: &HttpRequest) {
    debug!("All request headers:");
    for (name, value) in req.headers() {
        debug!("{}: {:?}", name, value);
    }
}


pub fn format_address_for_challenge(ip: IpAddr, port: i32) -> Result<SocketAddr, String> {
    match ip {
        IpAddr::V4(_) => {
            format!("{}:{}", ip, port)
                .parse()
                .map_err(|e| format!("Invalid socket address: {}", e))
        },
        IpAddr::V6(_) => Err("IPv6 addresses are not supported".to_string())
    }
}
