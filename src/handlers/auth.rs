use actix_web::{ web, HttpResponse, HttpRequest };
use capnp::message::ReaderOptions;
use log::{ debug, error };
use serde_json::json;
use std::net::{ IpAddr, SocketAddr };
use crate::storage::memory::ServerStorage;
use crate::models::server::{ ServerInfo, Player };
use crate::schema::server_heartbeat;
use governor::{ RateLimiter, clock::DefaultClock };
use governor::state::keyed::DefaultKeyedStateStore;
use crate::utils::{ extract_real_ip, format_address_for_challenge, RequestError, log_all_headers };
use tokio::net::UdpSocket;
use rand::Rng;
use std::fmt::Write;

pub async fn handle_auth(
    req: HttpRequest,
    storage: web::Data<ServerStorage>
) -> Result<HttpResponse, RequestError> {
    // check for code in query string
    let code = match req.query_string().split("code=").last() {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::BadRequest().body("Missing code in query string"));
        }
    };

    /*
    
     const tokenResponse = await axios.post(
    "https://discord.com/api/oauth2/token",
    new URLSearchParams({
      client_id: clientId,
      client_secret: clientSecret,
      grant_type: "authorization_code",
      code,
      redirect_uri: redirectUri,
      scope: scope,
    }),
    {
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
    }
  );
     */

    // send a request to discord to get the token
    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap();

    let client_secret = std::env::var("DISCORD_CLIENT_SECRET").unwrap().to_string();

    let redirect_uri = std::env::var("DISCORD_REDIRECT_URI").unwrap().to_string();

    let scope = "guilds.members.read";

    let token_response = reqwest::Client
        ::new()
        .post("https://discord.com/api/oauth2/token")
        .form(
            &[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("grant_type", "authorization_code".to_string()),
                ("code", code.to_string()),
                ("redirect_uri", redirect_uri),
                ("scope", scope.to_string()),
            ]
        )
        .send().await;

    let token_response = match token_response {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to get token from discord: {}", e);
            return Ok(HttpResponse::BadRequest().body("Failed to get token from discord"));
        }
    };

    let token_response = token_response.json::<serde_json::Value>().await;

    let token_response = match token_response {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to parse token response: {}", e);
            return Err(RequestError::AuthFailed);
        }
    };

    let refresh_token = match token_response.get("refresh_token") {
        Some(t) => t,
        None => {
            error!("Failed to get refresh token from discord response");
            return Err(RequestError::AuthFailed);
        }
    };

    let token_time = match token_response.get("expires_in") {
        Some(t) => t,
        None => {
            error!("Failed to get token time from discord response");
            return Err(RequestError::AuthFailed);
        }
    };

    let token = match token_response.get("access_token") {
        Some(t) => t,
        None => {
            error!("Failed to get access token from discord response");
            return Err(RequestError::AuthFailed);
        }
    };

    // get user info
    let guild_info = reqwest::Client
        ::new()
        .get("https://discord.com/api/users/@me/guilds/1186901921567617115/member")
        .bearer_auth(token.as_str().unwrap())
        .send().await;

    let guild_info = match guild_info {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to get guild info from discord: {}", e);
            return Err(RequestError::AuthFailed);
        }
    };

    let guild_info = guild_info.json::<serde_json::Value>().await;

    let guild_info = match guild_info {
        Ok(j) => j,
        Err(e) => {
            error!("Failed to parse guild info: {}", e);
            return Err(RequestError::AuthFailed);
        }
    };

    println!("Guild info: {:?}", guild_info);

    if !guild_info.get("roles").is_some() {
        return Ok(HttpResponse::BadRequest().body("You are not in the correct guild"));
    }

    // strip guild roles and id from user info
    let mut discord_id = String::new();

    let mut roles = Vec::new();
    let role_vec = guild_info.get("roles").unwrap().as_array().unwrap();
    for role in role_vec {
        roles.push(role.as_str().unwrap().to_string());
    }
    // guild info is for one guild and roles is an array in it so get the roles key

    if let Some(id) = guild_info.get("user").unwrap().get("id") {
        discord_id = id.as_str().unwrap().to_string();
    }

    // check for 1214775914836008990 playtester role

    if !roles.contains(&"1214775914836008990".to_string()) {
        return Ok(HttpResponse::BadRequest().body("Please verify on the R1Delta discord"));
    }

    println!("Discord ID: {}", discord_id);
    println!("Roles: {:?}", roles);

    let username = match guild_info.get("user").unwrap().get("username") {
        Some(u) => u.as_str().unwrap(),
        None => {
            return Ok(HttpResponse::BadRequest().body("Failed to get username from discord"));
        }
    };

    println!("Username: {}", username);

    Ok(HttpResponse::Ok().body("ok"))
}
