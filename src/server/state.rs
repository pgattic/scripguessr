use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tower_sessions::session_store::ExpiredDeletion;
use tower_sessions_sqlx_store::PostgresStore;

use crate::scriptures::ScriptureLibrary;

const GAME_CLEANUP_INTERVAL: Duration = Duration::from_secs(15 * 60);
const SESSION_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);

#[derive(Clone)]
pub struct AppState {
    pub library: Arc<ScriptureLibrary>,
    pub pool: PgPool,
    game_ttl: Duration,
}

impl AppState {
    pub fn new(pool: PgPool, game_ttl: Duration) -> Result<Self, serde_json::Error> {
        Ok(Self {
            library: Arc::new(ScriptureLibrary::standard_works()?),
            pool,
            game_ttl,
        })
    }

    /// Deletes abandoned games. Finished games of signed-in players are kept for stats.
    pub async fn prune_games(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM games WHERE (NOT finished OR user_id IS NULL) AND last_seen_at < now() - ($1 * interval '1 second')",
        )
        .bind(self.game_ttl.as_secs() as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub fn spawn_game_cleanup(&self) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(GAME_CLEANUP_INTERVAL);
            interval.tick().await;
            loop {
                interval.tick().await;
                if let Err(error) = state.prune_games().await {
                    tracing::error!(%error, "could not prune expired games");
                }
            }
        });
    }
}

pub fn spawn_session_cleanup(store: PostgresStore) {
    tokio::spawn(async move {
        if let Err(error) = store
            .continuously_delete_expired(SESSION_CLEANUP_INTERVAL)
            .await
        {
            tracing::error!(%error, "expired session cleanup stopped");
        }
    });
}
