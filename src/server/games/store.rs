use sqlx::types::Json as DbJson;
use sqlx::{FromRow, PgConnection, PgExecutor};
use uuid::Uuid;

use super::rounds::RoundAnswer;
use crate::api::GuessResponse;
use crate::scoring::max_total_score;
use crate::scriptures::{Difficulty, GameScope};
use crate::server::ApiError;
use crate::study_sets::StudyPassage;

const SELECT_GAME: &str = "SELECT user_id, difficulty, scope, playable_verse_count, round_count, current_round_index, finished FROM games WHERE id = $1";

#[derive(FromRow)]
pub struct DbGame {
    pub user_id: Option<Uuid>,
    pub difficulty: DbJson<Difficulty>,
    pub scope: DbJson<GameScope>,
    pub playable_verse_count: i32,
    pub round_count: i32,
    pub current_round_index: i32,
    pub finished: bool,
}

#[derive(FromRow)]
pub struct DbRound {
    pub text: String,
    pub passage: DbJson<StudyPassage>,
    pub source_passage: DbJson<StudyPassage>,
    pub guess: Option<DbJson<GuessResponse>>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum GameKind {
    Random,
    Study,
}

impl GameKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Random => "random",
            Self::Study => "study",
        }
    }
}

pub struct NewGame {
    pub kind: GameKind,
    pub leaderboard_preset: Option<&'static str>,
    pub difficulty: Difficulty,
    pub scope: GameScope,
    pub playable_verse_count: usize,
    pub rounds: Vec<RoundAnswer>,
}

#[derive(Clone, Copy)]
pub enum Lock {
    None,
    ForUpdate,
}

pub async fn fetch_game<'e>(
    conn: impl PgExecutor<'e>,
    id: Uuid,
    lock: Lock,
) -> Result<DbGame, ApiError> {
    let query = match lock {
        Lock::None => SELECT_GAME.to_string(),
        Lock::ForUpdate => format!("{SELECT_GAME} FOR UPDATE"),
    };
    sqlx::query_as::<_, DbGame>(&query)
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(game_not_found)
}

pub async fn fetch_rounds<'e>(
    conn: impl PgExecutor<'e>,
    id: Uuid,
) -> Result<Vec<DbRound>, ApiError> {
    Ok(sqlx::query_as::<_, DbRound>(
        "SELECT text, passage, source_passage, guess FROM game_rounds WHERE game_id = $1 ORDER BY round_index",
    )
    .bind(id)
    .fetch_all(conn)
    .await?)
}

pub async fn fetch_round<'e>(
    conn: impl PgExecutor<'e>,
    id: Uuid,
    index: i32,
) -> Result<Option<DbRound>, ApiError> {
    Ok(sqlx::query_as::<_, DbRound>(
        "SELECT text, passage, source_passage, guess FROM game_rounds WHERE game_id = $1 AND round_index = $2",
    )
    .bind(id)
    .bind(index)
    .fetch_optional(conn)
    .await?)
}

pub async fn insert_game(
    conn: &mut PgConnection,
    id: Uuid,
    user_id: Option<Uuid>,
    game: &NewGame,
) -> Result<(), ApiError> {
    let round_count = game.rounds.len();
    sqlx::query(
        "INSERT INTO games (id, user_id, game_kind, leaderboard_preset, difficulty, scope, playable_verse_count, round_count, possible_score) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(id)
    .bind(user_id)
    .bind(game.kind.as_str())
    .bind(game.leaderboard_preset)
    .bind(DbJson(game.difficulty))
    .bind(DbJson(&game.scope))
    .bind(game.playable_verse_count as i32)
    .bind(round_count as i32)
    .bind(max_total_score(round_count) as i32)
    .execute(&mut *conn)
    .await?;
    for (index, round) in game.rounds.iter().enumerate() {
        sqlx::query(
            "INSERT INTO game_rounds (game_id, round_index, text, passage, source_passage) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(index as i32)
        .bind(&round.text)
        .bind(DbJson(&round.passage))
        .bind(DbJson(&round.source_passage))
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub async fn save_guess<'e>(
    conn: impl PgExecutor<'e>,
    id: Uuid,
    index: i32,
    guess: &GuessResponse,
) -> Result<(), ApiError> {
    sqlx::query("UPDATE game_rounds SET guess = $1 WHERE game_id = $2 AND round_index = $3")
        .bind(DbJson(guess))
        .bind(id)
        .bind(index)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn round_guessed<'e>(
    conn: impl PgExecutor<'e>,
    id: Uuid,
    index: i32,
) -> Result<bool, ApiError> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT guess IS NOT NULL FROM game_rounds WHERE game_id = $1 AND round_index = $2",
    )
    .bind(id)
    .bind(index)
    .fetch_one(conn)
    .await?)
}

pub async fn touch_game<'e>(conn: impl PgExecutor<'e>, id: Uuid) -> Result<(), ApiError> {
    sqlx::query("UPDATE games SET last_seen_at = now() WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn set_round_index<'e>(
    conn: impl PgExecutor<'e>,
    id: Uuid,
    index: i32,
) -> Result<(), ApiError> {
    sqlx::query("UPDATE games SET current_round_index = $1, last_seen_at = now() WHERE id = $2")
        .bind(index)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn finish_game<'e>(conn: impl PgExecutor<'e>, id: Uuid) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE games SET finished = true, completed_at = now(), last_seen_at = now(), score = (SELECT COALESCE(SUM((guess->'score'->>'points')::integer), 0) FROM game_rounds WHERE game_id = $1) WHERE id = $1",
    )
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub fn game_not_found() -> ApiError {
    ApiError::not_found("Game not found or expired")
}
