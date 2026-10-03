mod account;
mod auth;
mod catalog;
mod config;
mod error;
mod games;
mod leaderboards;
mod state;
mod validation;

use std::net::SocketAddr;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowHeaders, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::EnvFilter;

use self::config::Config;
pub use self::error::ApiError;
use self::state::{AppState, spawn_session_cleanup};

const MAX_REQUEST_BYTES: usize = 512 * 1024;

pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let config = Config::from_env()?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    let session_store = PostgresStore::new(pool.clone());
    session_store.migrate().await?;
    spawn_session_cleanup(session_store.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name("scripguessr.sid")
        .with_same_site(SameSite::Lax)
        .with_secure(config.secure_cookies)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(30)));
    let state = AppState::new(pool, config.game_ttl)?;
    state.prune_games().await?;
    state.spawn_game_cleanup();

    let app = router(state, &config.static_dir, config.allowed_origins).layer(session_layer);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn router(state: AppState, static_dir: &str, allowed_origins: Vec<HeaderValue>) -> Router {
    let static_files = ServeDir::new(static_dir)
        .append_index_html_on_directories(true)
        .not_found_service(ServeFile::new(format!("{static_dir}/index.html")));

    let router = Router::new()
        .merge(catalog::routes())
        .merge(games::routes())
        .merge(auth::routes())
        .merge(account::routes())
        .merge(leaderboards::routes())
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .fallback_service(static_files)
        .with_state(state);
    if allowed_origins.is_empty() {
        return router;
    }
    router.layer(
        CorsLayer::new()
            .allow_origin(allowed_origins)
            .allow_credentials(true)
            .allow_headers(AllowHeaders::mirror_request())
            .allow_methods([Method::GET, Method::POST, Method::PUT]),
    )
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("scripguessr=info,tower_http=info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
