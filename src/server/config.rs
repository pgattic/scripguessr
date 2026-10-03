use std::time::Duration;

use axum::http::HeaderValue;

const DEFAULT_PORT: u16 = 8087;
const DEFAULT_GAME_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const DEFAULT_STATIC_DIR: &str = "dist/public";

pub struct Config {
    pub port: u16,
    pub game_ttl: Duration,
    pub static_dir: String,
    pub database_url: String,
    pub secure_cookies: bool,
    pub allowed_origins: Vec<HeaderValue>,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            port: env_parse("PORT").unwrap_or(DEFAULT_PORT),
            game_ttl: env_parse("SCRIPGUESSR_GAME_TTL_SECONDS")
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_GAME_TTL),
            static_dir: std::env::var("SCRIPGUESSR_STATIC_DIR")
                .unwrap_or_else(|_| DEFAULT_STATIC_DIR.to_string()),
            database_url: database_url()?,
            secure_cookies: std::env::var("SCRIPGUESSR_SECURE_COOKIES")
                .map(|value| value != "false" && value != "0")
                .unwrap_or(true),
            allowed_origins: std::env::var("SCRIPGUESSR_ALLOWED_ORIGIN")
                .map(|origins| {
                    origins
                        .split(',')
                        .filter_map(|origin| origin.trim().parse().ok())
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

fn env_parse<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok()?.parse().ok()
}

fn database_url() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("SCRIPGUESSR_DATABASE_URL_FILE") {
        return Ok(std::fs::read_to_string(path)?.trim().to_string());
    }
    Ok(std::env::var("DATABASE_URL")?)
}
