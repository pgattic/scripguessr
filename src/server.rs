use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Json as DbJson;
use sqlx::{FromRow, PgPool};
use tower_http::cors::{AllowHeaders, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tower_sessions::{
    Expiry, Session, SessionManagerLayer, cookie::SameSite, session_store::ExpiredDeletion,
};
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod account;
mod auth;
mod leaderboards;

use crate::api::{
    AdvanceGameResponse, CanonMetadata, ChapterRequest, ChapterResponse, ChapterVerse,
    GameSnapshotResponse, GuessReference, GuessRequest, GuessResponse, MetadataRequest,
    MetadataResponse, NewGameRequest, NewGameResponse, RoundPrompt, SnapshotRound,
};
use crate::scriptures::{Canon, ScriptureLibrary, Scriptures, Verse};

const BOOK_OF_MORMON_DATA: &str = include_str!("../assets/data/book-of-mormon-flat.json");
const DOCTRINE_AND_COVENANTS_DATA: &str =
    include_str!("../assets/data/doctrine-and-covenants-flat.json");
const PEARL_OF_GREAT_PRICE_DATA: &str =
    include_str!("../assets/data/pearl-of-great-price-flat.json");
const OLD_TESTAMENT_DATA: &str = include_str!("../assets/data/old-testament-flat.json");
const NEW_TESTAMENT_DATA: &str = include_str!("../assets/data/new-testament-flat.json");
const DEFAULT_GAME_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(15 * 60);
const SESSION_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
const MAX_GAME_ROUNDS: usize = 1_000;
const MAX_STUDY_PASSAGES: usize = 2_000;
const MAX_VERSES_PER_PASSAGE: usize = 200;
const MAX_GAME_REQUEST_BYTES: usize = 512 * 1024;

pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(8087);
    let game_ttl = std::env::var("SCRIPGUESSR_GAME_TTL_SECONDS")
        .ok()
        .and_then(|ttl| ttl.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_GAME_TTL);
    let static_dir =
        std::env::var("SCRIPGUESSR_STATIC_DIR").unwrap_or_else(|_| "dist/public".to_string());
    let database_url = database_url()?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    let session_store = PostgresStore::new(pool.clone());
    session_store.migrate().await?;
    spawn_session_cleanup(session_store.clone());
    let secure_cookies = std::env::var("SCRIPGUESSR_SECURE_COOKIES")
        .map(|value| value != "false" && value != "0")
        .unwrap_or(true);
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name("scripguessr.sid")
        .with_same_site(SameSite::Lax)
        .with_secure(secure_cookies)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(30)));
    let state = AppState::new(pool, game_ttl)?;
    state.prune_games().await?;
    spawn_game_cleanup(state.clone());

    let app = router_for(state, static_dir).layer(session_layer);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn database_url() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("SCRIPGUESSR_DATABASE_URL_FILE") {
        return Ok(std::fs::read_to_string(path)?.trim().to_string());
    }
    Ok(std::env::var("DATABASE_URL")?)
}

fn router_for(state: AppState, static_dir: impl Into<String>) -> Router {
    let static_dir = static_dir.into();
    let index_file = format!("{static_dir}/index.html");
    let static_files = ServeDir::new(static_dir)
        .append_index_html_on_directories(true)
        .not_found_service(ServeFile::new(index_file));

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .merge(auth::routes())
        .merge(account::routes())
        .merge(leaderboards::routes())
        .route("/api/metadata", post(metadata))
        .route("/api/chapter", post(chapter))
        .route("/api/games", post(create_game))
        .route("/api/games/{game_id}", get(game_snapshot))
        .route("/api/games/{game_id}/guesses", post(submit_guess))
        .route("/api/games/{game_id}/advance", post(advance_game))
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(MAX_GAME_REQUEST_BYTES))
        .fallback_service(static_files)
        .with_state(state);
    if let Ok(origin) = std::env::var("SCRIPGUESSR_ALLOWED_ORIGIN")
        && let Ok(origin) = origin.parse::<HeaderValue>()
    {
        return router.layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_credentials(true)
                .allow_headers(AllowHeaders::mirror_request())
                .allow_methods([Method::GET, Method::POST, Method::PUT]),
        );
    }
    router
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("scripguessr=info,tower_http=info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

#[derive(Clone)]
struct AppState {
    library: Arc<ScriptureLibrary>,
    pool: PgPool,
    game_ttl: Duration,
}

impl AppState {
    fn new(pool: PgPool, game_ttl: Duration) -> Result<Self, serde_json::Error> {
        let mut library = ScriptureLibrary::default();
        for (canon, data) in [
            (Canon::OldTestament, OLD_TESTAMENT_DATA),
            (Canon::NewTestament, NEW_TESTAMENT_DATA),
            (Canon::BookOfMormon, BOOK_OF_MORMON_DATA),
            (Canon::DoctrineAndCovenants, DOCTRINE_AND_COVENANTS_DATA),
            (Canon::PearlOfGreatPrice, PEARL_OF_GREAT_PRICE_DATA),
        ] {
            library.insert(canon, Scriptures::from_flat_json_for_canon(canon, data)?);
        }

        Ok(Self {
            library: Arc::new(library),
            pool,
            game_ttl,
        })
    }

    async fn prune_games(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM games WHERE (NOT finished OR user_id IS NULL) AND last_seen_at < now() - ($1 * interval '1 second')",
        )
        .bind(self.game_ttl.as_secs() as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn spawn_game_cleanup(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = state.prune_games().await {
                tracing::error!(%error, "could not prune expired games");
            }
        }
    });
}

fn spawn_session_cleanup(store: PostgresStore) {
    tokio::spawn(async move {
        if let Err(error) = store
            .continuously_delete_expired(SESSION_CLEANUP_INTERVAL)
            .await
        {
            tracing::error!(%error, "expired session cleanup stopped");
        }
    });
}

#[derive(Clone)]
struct RoundAnswer {
    passage: crate::study_sets::StudyPassage,
    source_passage: crate::study_sets::StudyPassage,
    text: String,
}

#[derive(FromRow)]
struct DbGame {
    user_id: Option<Uuid>,
    difficulty: DbJson<crate::scriptures::Difficulty>,
    scope: DbJson<crate::scriptures::GameScope>,
    playable_verse_count: i32,
    round_count: i32,
    current_round_index: i32,
    finished: bool,
}

#[derive(FromRow)]
struct DbRound {
    #[sqlx(rename = "round_index")]
    _round_index: i32,
    text: String,
    passage: DbJson<crate::study_sets::StudyPassage>,
    source_passage: DbJson<crate::study_sets::StudyPassage>,
    guess: Option<DbJson<GuessResponse>>,
}

impl From<Verse> for RoundAnswer {
    fn from(verse: Verse) -> Self {
        let passage = crate::study_sets::StudyPassage::single(&verse.reference);
        Self {
            source_passage: passage.clone(),
            passage,
            text: verse.text,
        }
    }
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(state): State<AppState>) -> Result<&'static str, ApiError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::database)?;
    Ok("ready")
}

async fn metadata(
    State(state): State<AppState>,
    Json(request): Json<MetadataRequest>,
) -> Result<Json<MetadataResponse>, ApiError> {
    if request.scope.canons.is_empty() {
        return Err(ApiError::bad_request(
            "At least one scope item must be selected",
        ));
    }

    Ok(Json(MetadataResponse {
        difficulty: request.difficulty,
        scope: request.scope.clone(),
        metadata: metadata_for(&state.library, request.difficulty, &request.scope),
        playable_verse_count: playable_verse_count(
            &state.library,
            request.difficulty,
            &request.scope,
        ),
        total_verse_count: total_verse_count(&state.library, &request.scope),
    }))
}

async fn chapter(
    State(state): State<AppState>,
    Json(request): Json<ChapterRequest>,
) -> Result<Json<ChapterResponse>, ApiError> {
    let reference = crate::scriptures::Reference {
        canon: request.canon,
        book: request.book,
        chapter: request.chapter,
        verse: 1,
    };
    let verses = state
        .library
        .scriptures(reference.canon)
        .ok_or_else(|| ApiError::not_found("Canon not found"))?
        .verses_for_chapter(&reference);
    if verses.is_empty() {
        return Err(ApiError::not_found("Chapter not found"));
    }
    Ok(Json(ChapterResponse {
        verses: verses
            .into_iter()
            .map(|verse| ChapterVerse {
                verse: verse.reference.verse,
                text: verse.text,
            })
            .collect(),
    }))
}

async fn create_game(
    State(state): State<AppState>,
    session: Session,
    Json(request): Json<NewGameRequest>,
) -> Result<Json<NewGameResponse>, ApiError> {
    let mut rng = SmallRng::from_os_rng();
    let (game_kind, difficulty, scope, rounds, playable_count) = match request {
        NewGameRequest::Random {
            round_count,
            difficulty,
            scope,
        } => {
            validate_game_basics(round_count, &scope)?;
            let playable_count = playable_verse_count(&state.library, difficulty, &scope);
            if playable_count == 0 {
                return Err(ApiError::bad_request("No playable verses found"));
            }
            let mut rounds = Vec::with_capacity(round_count);
            for _round in 0..round_count {
                rounds.push(
                    random_verse(&state.library, difficulty, &scope, playable_count, &mut rng)?
                        .into(),
                );
            }
            ("random", difficulty, scope, rounds, playable_count)
        }
        NewGameRequest::Study {
            round_count,
            difficulty,
            scope,
            passages,
            prompt_policy,
        } => {
            validate_game_basics(round_count, &scope)?;
            validate_study_passages(&state.library, &scope, &passages)?;
            let rounds = study_passages(
                &state.library,
                passages,
                round_count,
                prompt_policy,
                &mut rng,
            )?;
            let playable_count = rounds.len();
            ("study", difficulty, scope, rounds, playable_count)
        }
    };
    let round_count = rounds.len();
    let user_id = auth::optional_user_id(&session).await?;
    let leaderboard_preset = (game_kind == "random")
        .then(|| leaderboard_preset(&scope, round_count))
        .flatten();
    let game_uuid = Uuid::new_v4();
    let mut tx = state.pool.begin().await.map_err(ApiError::database)?;
    sqlx::query(
        "INSERT INTO games (id, user_id, game_kind, leaderboard_preset, difficulty, scope, playable_verse_count, round_count, possible_score) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(game_uuid)
    .bind(user_id)
    .bind(game_kind)
    .bind(leaderboard_preset)
    .bind(DbJson(difficulty))
    .bind(DbJson(&scope))
    .bind(playable_count as i32)
    .bind(round_count as i32)
    .bind(round_count as i32 * crate::scoring::MAX_SCORE as i32)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::database)?;
    for (index, round) in rounds.iter().enumerate() {
        sqlx::query(
            "INSERT INTO game_rounds (game_id, round_index, text, passage, source_passage) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(game_uuid)
        .bind(index as i32)
        .bind(&round.text)
        .bind(DbJson(&round.passage))
        .bind(DbJson(&round.source_passage))
        .execute(&mut *tx)
        .await
        .map_err(ApiError::database)?;
    }
    tx.commit().await.map_err(ApiError::database)?;
    let game_id = game_uuid.to_string();

    Ok(Json(NewGameResponse {
        game_id,
        rounds: rounds
            .iter()
            .map(|round| RoundPrompt {
                text: round.text.clone(),
            })
            .collect(),
        difficulty,
        scope: scope.clone(),
        metadata: metadata_for(&state.library, difficulty, &scope),
        playable_verse_count: playable_count,
        total_verse_count: total_verse_count(&state.library, &scope),
        max_total_score: round_count as u32 * crate::scoring::MAX_SCORE,
    }))
}

async fn game_snapshot(
    State(state): State<AppState>,
    session: Session,
    Path(game_id): Path<String>,
) -> Result<Json<GameSnapshotResponse>, ApiError> {
    let game_uuid = parse_game_id(&game_id)?;
    let game = sqlx::query_as::<_, DbGame>(
        "SELECT user_id, difficulty, scope, playable_verse_count, round_count, current_round_index, finished FROM games WHERE id = $1",
    )
    .bind(game_uuid)
    .fetch_optional(&state.pool)
    .await
    .map_err(ApiError::database)?
    .ok_or_else(|| ApiError::not_found("Game not found or expired"))?;
    ensure_game_access(game.user_id, auth::optional_user_id(&session).await?)?;
    let rounds = sqlx::query_as::<_, DbRound>(
        "SELECT round_index, text, passage, source_passage, guess FROM game_rounds WHERE game_id = $1 ORDER BY round_index",
    )
    .bind(game_uuid)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::database)?;
    sqlx::query("UPDATE games SET last_seen_at = now() WHERE id = $1")
        .bind(game_uuid)
        .execute(&state.pool)
        .await
        .map_err(ApiError::database)?;

    Ok(Json(GameSnapshotResponse {
        game_id,
        rounds: rounds
            .into_iter()
            .map(|round| SnapshotRound {
                text: round.text,
                guess: round.guess.map(|guess| guess.0),
            })
            .collect(),
        current_round_index: game.current_round_index as usize,
        finished: game.finished,
        difficulty: game.difficulty.0,
        scope: game.scope.0.clone(),
        metadata: metadata_for(&state.library, game.difficulty.0, &game.scope.0),
        playable_verse_count: game.playable_verse_count as usize,
        total_verse_count: total_verse_count(&state.library, &game.scope.0),
        max_total_score: game.round_count as u32 * crate::scoring::MAX_SCORE,
    }))
}

async fn submit_guess(
    State(state): State<AppState>,
    session: Session,
    Path(game_id): Path<String>,
    Json(request): Json<GuessRequest>,
) -> Result<Json<GuessResponse>, ApiError> {
    let game_uuid = parse_game_id(&game_id)?;
    let mut tx = state.pool.begin().await.map_err(ApiError::database)?;
    let game = sqlx::query_as::<_, DbGame>(
        "SELECT user_id, difficulty, scope, playable_verse_count, round_count, current_round_index, finished FROM games WHERE id = $1 FOR UPDATE",
    )
    .bind(game_uuid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::database)?
    .ok_or_else(|| ApiError::not_found("Game not found"))?;
    ensure_game_access(game.user_id, auth::optional_user_id(&session).await?)?;
    if game.finished || request.round_index != game.current_round_index as usize {
        return Err(ApiError::bad_request("Round is not active"));
    }
    let round = sqlx::query_as::<_, DbRound>(
        "SELECT round_index, text, passage, source_passage, guess FROM game_rounds WHERE game_id = $1 AND round_index = $2",
    )
    .bind(game_uuid)
    .bind(request.round_index as i32)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::database)?
    .ok_or_else(|| ApiError::bad_request("Round not found"))?;
    if let Some(guess) = round.guess {
        return Ok(Json(guess.0));
    }
    let scope = game.scope.0;
    let answer = round.passage.0;
    let answer_reference = answer
        .first_reference()
        .ok_or_else(|| ApiError::bad_request("Round has no answer verses"))?;
    let score = state.library.score(
        &scope,
        &answer_reference,
        request.canon,
        &request.book,
        request.chapter,
    );
    let chapter_verses = state
        .library
        .scriptures(answer.canon)
        .ok_or_else(|| ApiError::not_found("Canon not found"))?
        .verses_for_chapter(&answer_reference)
        .into_iter()
        .map(|verse| ChapterVerse {
            verse: verse.reference.verse,
            text: verse.text,
        })
        .collect();

    let response = GuessResponse {
        answer,
        source_passage: round.source_passage.0,
        guess: GuessReference {
            canon: request.canon,
            book: request.book,
            chapter: request.chapter,
        },
        score,
        chapter_verses,
    };
    sqlx::query("UPDATE game_rounds SET guess = $1 WHERE game_id = $2 AND round_index = $3")
        .bind(DbJson(&response))
        .bind(game_uuid)
        .bind(request.round_index as i32)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::database)?;
    sqlx::query("UPDATE games SET last_seen_at = now() WHERE id = $1")
        .bind(game_uuid)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::database)?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok(Json(response))
}

async fn advance_game(
    State(state): State<AppState>,
    session: Session,
    Path(game_id): Path<String>,
) -> Result<Json<AdvanceGameResponse>, ApiError> {
    let game_uuid = parse_game_id(&game_id)?;
    let mut tx = state.pool.begin().await.map_err(ApiError::database)?;
    let game = sqlx::query_as::<_, DbGame>(
        "SELECT user_id, difficulty, scope, playable_verse_count, round_count, current_round_index, finished FROM games WHERE id = $1 FOR UPDATE",
    )
    .bind(game_uuid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::database)?
    .ok_or_else(|| ApiError::not_found("Game not found or expired"))?;
    ensure_game_access(game.user_id, auth::optional_user_id(&session).await?)?;
    if game.finished {
        return Ok(Json(AdvanceGameResponse {
            current_round_index: game.current_round_index as usize,
            finished: true,
        }));
    }
    let guessed = sqlx::query_scalar::<_, bool>(
        "SELECT guess IS NOT NULL FROM game_rounds WHERE game_id = $1 AND round_index = $2",
    )
    .bind(game_uuid)
    .bind(game.current_round_index)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::database)?;
    if !guessed {
        return Err(ApiError::bad_request("Submit a guess before advancing"));
    }

    let (current_round_index, finished) = if game.current_round_index + 1 == game.round_count {
        let score = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM((guess->'score'->>'points')::integer), 0::bigint) FROM game_rounds WHERE game_id = $1",
        )
        .bind(game_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(ApiError::database)?;
        sqlx::query("UPDATE games SET finished = true, score = $1, completed_at = now(), last_seen_at = now() WHERE id = $2")
            .bind(score as i32)
            .bind(game_uuid)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::database)?;
        (game.current_round_index, true)
    } else {
        let next = game.current_round_index + 1;
        sqlx::query(
            "UPDATE games SET current_round_index = $1, last_seen_at = now() WHERE id = $2",
        )
        .bind(next)
        .bind(game_uuid)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::database)?;
        (next, false)
    };
    tx.commit().await.map_err(ApiError::database)?;

    Ok(Json(AdvanceGameResponse {
        current_round_index: current_round_index as usize,
        finished,
    }))
}

fn parse_game_id(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| ApiError::not_found("Game not found"))
}

fn ensure_game_access(owner: Option<Uuid>, current_user: Option<Uuid>) -> Result<(), ApiError> {
    if owner.is_some() && owner != current_user {
        return Err(ApiError::unauthorized(
            "This game belongs to another account",
        ));
    }
    Ok(())
}

fn leaderboard_preset(
    scope: &crate::scriptures::GameScope,
    round_count: usize,
) -> Option<&'static str> {
    if !matches!(round_count, 5 | 10) {
        return None;
    }
    crate::scriptures::GameMode::ALL
        .into_iter()
        .find(|mode| mode.scope() == *scope)
        .map(game_mode_key)
}

fn game_mode_key(mode: crate::scriptures::GameMode) -> &'static str {
    match mode {
        crate::scriptures::GameMode::BookOfMormon => "book_of_mormon",
        crate::scriptures::GameMode::Bible => "bible",
        crate::scriptures::GameMode::Restoration => "restoration",
        crate::scriptures::GameMode::AllStandardWorks => "all_standard_works",
    }
}

fn random_verse(
    library: &ScriptureLibrary,
    difficulty: crate::scriptures::Difficulty,
    scope: &crate::scriptures::GameScope,
    playable_count: usize,
    rng: &mut SmallRng,
) -> Result<Verse, ApiError> {
    let mut index = rng.random_range(0..playable_count);

    for canon_scope in &scope.canons {
        let scriptures = library
            .scriptures(canon_scope.canon)
            .ok_or_else(|| ApiError::bad_request("Unknown canon"))?;
        let count = scriptures.scoped_verse_count_for_difficulty(difficulty, &canon_scope.books);
        if index < count {
            return scriptures
                .scoped_verse_for_difficulty(difficulty, &canon_scope.books, index)
                .cloned()
                .ok_or_else(|| ApiError::bad_request("No playable verses found"));
        }
        index -= count;
    }

    Err(ApiError::bad_request("No playable verses found"))
}

fn study_passages(
    library: &ScriptureLibrary,
    passages: Vec<crate::study_sets::StudyPassage>,
    round_count: usize,
    prompt_policy: crate::study_sets::PromptPolicy,
    rng: &mut SmallRng,
) -> Result<Vec<RoundAnswer>, ApiError> {
    let mut seen = HashSet::with_capacity(passages.len());
    let mut selected = Vec::with_capacity(passages.len().min(round_count));
    for passage in passages {
        if seen.insert(passage.clone()) {
            selected.push(passage);
        }
    }
    selected.shuffle(rng);
    selected.truncate(round_count);

    let mut rounds = Vec::with_capacity(selected.len());
    for passage in selected {
        let shown_passage = if prompt_policy.shows_whole_passage(&passage) {
            passage.clone()
        } else {
            let index = rng.random_range(0..passage.verses.len());
            crate::study_sets::StudyPassage {
                canon: passage.canon,
                book: passage.book.clone(),
                chapter: passage.chapter,
                verses: vec![passage.verses[index]],
            }
        };
        let verse = library
            .scriptures(passage.canon)
            .and_then(|scriptures| {
                scriptures.passage(
                    &shown_passage.book,
                    shown_passage.chapter,
                    &shown_passage.verses,
                )
            })
            .ok_or_else(|| ApiError::bad_request("Study passage was not found"))?;
        rounds.push(RoundAnswer {
            passage: shown_passage,
            source_passage: passage,
            text: verse.text,
        });
    }

    if rounds.is_empty() {
        return Err(ApiError::bad_request("No study passages found"));
    }
    Ok(rounds)
}

fn validate_game_basics(
    round_count: usize,
    scope: &crate::scriptures::GameScope,
) -> Result<(), ApiError> {
    if scope.canons.is_empty() || round_count == 0 {
        return Err(ApiError::bad_request("Game must include rounds and scope"));
    }
    if round_count > MAX_GAME_ROUNDS {
        return Err(ApiError::bad_request("Too many rounds requested"));
    }
    Ok(())
}

fn validate_study_passages(
    library: &ScriptureLibrary,
    scope: &crate::scriptures::GameScope,
    passages: &[crate::study_sets::StudyPassage],
) -> Result<(), ApiError> {
    if passages.is_empty() {
        return Err(ApiError::bad_request("Study game must include passages"));
    }
    if passages.len() > MAX_STUDY_PASSAGES {
        return Err(ApiError::bad_request("Too many study passages"));
    }
    for passage in passages {
        if !scope.includes_book(passage.canon, &passage.book) {
            return Err(ApiError::bad_request(
                "Study passage is outside the game scope",
            ));
        }
        if passage.verses.is_empty() || passage.verses.len() > MAX_VERSES_PER_PASSAGE {
            return Err(ApiError::bad_request(
                "Study passage has an invalid verse count",
            ));
        }
        let exists = library.scriptures(passage.canon).is_some_and(|scriptures| {
            scriptures.contains_passage(&passage.book, passage.chapter, &passage.verses)
        });
        if !exists {
            return Err(ApiError::bad_request("Study passage was not found"));
        }
    }
    Ok(())
}

fn metadata_for(
    library: &ScriptureLibrary,
    difficulty: crate::scriptures::Difficulty,
    scope: &crate::scriptures::GameScope,
) -> Vec<CanonMetadata> {
    scope
        .canons
        .iter()
        .filter_map(|canon_scope| {
            let scriptures = library.scriptures(canon_scope.canon)?;
            Some(CanonMetadata {
                canon: canon_scope.canon,
                books: scriptures.books.clone(),
                playable_verse_count: scriptures
                    .scoped_verse_count_for_difficulty(difficulty, &canon_scope.books),
                total_verse_count: scriptures.scoped_total_verse_count(&canon_scope.books),
            })
        })
        .collect()
}

fn playable_verse_count(
    library: &ScriptureLibrary,
    difficulty: crate::scriptures::Difficulty,
    scope: &crate::scriptures::GameScope,
) -> usize {
    scope
        .canons
        .iter()
        .filter_map(|canon_scope| {
            library
                .scriptures(canon_scope.canon)
                .map(|scriptures| (scriptures, canon_scope))
        })
        .map(|(scriptures, canon_scope)| {
            scriptures.scoped_verse_count_for_difficulty(difficulty, &canon_scope.books)
        })
        .sum()
}

fn total_verse_count(library: &ScriptureLibrary, scope: &crate::scriptures::GameScope) -> usize {
    scope
        .canons
        .iter()
        .filter_map(|canon_scope| {
            library
                .scriptures(canon_scope.canon)
                .map(|scriptures| (scriptures, canon_scope))
        })
        .map(|(scriptures, canon_scope)| scriptures.scoped_total_verse_count(&canon_scope.books))
        .sum()
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    fn database(error: sqlx::Error) -> Self {
        tracing::error!(%error, "database request failed");
        Self::internal("Database request failed")
    }

    fn session(error: tower_sessions::session::Error) -> Self {
        tracing::error!(%error, "session request failed");
        Self::internal("Session request failed")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        #[derive(Serialize)]
        struct ErrorBody {
            error: String,
        }

        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

// Superseded by the PostgreSQL integration suite. Kept temporarily as migration
// history until each endpoint assertion has been moved to tests/backend.rs.
#[cfg(all(test, any()))]
mod tests {
    use super::*;
    use crate::scriptures::{BookScope, CanonScope, Difficulty, GameMode, GameScope};
    use crate::study_sets::built_in_study_sets;
    use axum::body::{Body, to_bytes};
    use axum::http::{Method, Request, header};
    use serde::de::DeserializeOwned;
    use tower::ServiceExt;

    fn test_request() -> NewGameRequest {
        NewGameRequest::Random {
            round_count: 1,
            difficulty: Difficulty::Normal,
            scope: GameMode::BookOfMormon.scope(),
        }
    }

    fn alma_scope() -> GameScope {
        GameScope {
            canons: vec![CanonScope {
                canon: Canon::BookOfMormon,
                books: BookScope::Selected(vec!["Alma".to_string()]),
            }],
        }
    }

    fn alma_request() -> NewGameRequest {
        NewGameRequest::Random {
            round_count: 1,
            difficulty: Difficulty::Normal,
            scope: alma_scope(),
        }
    }

    fn study_request(
        scope: GameScope,
        passages: Vec<crate::study_sets::StudyPassage>,
        prompt_policy: crate::study_sets::PromptPolicy,
    ) -> NewGameRequest {
        NewGameRequest::Study {
            round_count: passages.len(),
            difficulty: Difficulty::Normal,
            scope,
            passages,
            prompt_policy,
        }
    }

    fn empty_server_game(last_seen_at: SystemTime) -> ServerGame {
        ServerGame {
            scope: GameMode::BookOfMormon.scope(),
            rounds: Vec::new(),
            guesses: Vec::new(),
            current_round_index: 0,
            finished: false,
            difficulty: Difficulty::Normal,
            playable_verse_count: 0,
            last_seen_at,
        }
    }

    #[test]
    fn pruning_removes_only_expired_games() {
        let state = AppState::new(Duration::from_secs(60)).unwrap();
        state.games.lock().unwrap().insert(
            "expired".to_string(),
            empty_server_game(SystemTime::UNIX_EPOCH),
        );
        state
            .games
            .lock()
            .unwrap()
            .insert("active".to_string(), empty_server_game(SystemTime::now()));

        state.prune_games();

        let games = state.games.lock().unwrap();
        assert!(!games.contains_key("expired"));
        assert!(games.contains_key("active"));
    }

    #[test]
    fn every_curated_study_passage_resolves_from_scripture_data() {
        let state = AppState::new(Duration::from_secs(60)).unwrap();
        let mut rng = SmallRng::seed_from_u64(1);
        for set in built_in_study_sets() {
            let expected_count = set.passages.len();
            let verses = study_passages(
                &state.library,
                set.passages.clone(),
                expected_count,
                crate::study_sets::PromptPolicy::WholePassage,
                &mut rng,
            )
            .unwrap();
            assert_eq!(verses.len(), expected_count, "set: {}", set.name);
            assert!(
                verses.iter().all(|verse| !verse.text.is_empty()),
                "set: {}",
                set.name
            );
        }
    }

    #[test]
    fn every_atlas_span_resolves_from_scripture_data() {
        let state = AppState::new(Duration::from_secs(60)).unwrap();
        let scriptures = state.library.scriptures(Canon::BookOfMormon).unwrap();

        for layer in crate::atlas::LAYERS {
            for span in layer.spans {
                let book = scriptures
                    .books
                    .iter()
                    .find(|book| book.name == span.book)
                    .unwrap_or_else(|| panic!("atlas book not found: {}", span.book));
                assert!(
                    (span.start..=span.end).all(|chapter| book.chapters.contains(&chapter)),
                    "atlas span not found: {} {}-{}",
                    span.book,
                    span.start,
                    span.end
                );
            }
        }
    }

    fn app() -> (Router, AppState) {
        let state = AppState::new(Duration::from_secs(60)).unwrap();
        (router_for(state.clone(), "/tmp"), state)
    }

    async fn get(app: Router, path: &str) -> axum::response::Response {
        app.oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn post_json<RequestBody>(
        app: Router,
        path: &str,
        body: RequestBody,
    ) -> axum::response::Response
    where
        RequestBody: Serialize,
    {
        let body = serde_json::to_vec(&body).unwrap();
        app.oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn read_json<ResponseBody>(response: axum::response::Response) -> ResponseBody
    where
        ResponseBody: DeserializeOwned,
    {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn healthz_returns_ok() {
        let (app, _state) = app();

        let response = get(app, "/healthz").await;
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn metadata_endpoint_returns_books_and_counts() {
        let (app, _state) = app();
        let response = post_json(
            app,
            "/api/metadata",
            MetadataRequest {
                difficulty: Difficulty::Normal,
                scope: GameMode::BookOfMormon.scope(),
            },
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: MetadataResponse = read_json(response).await;
        assert_eq!(body.scope, GameMode::BookOfMormon.scope());
        assert_eq!(body.metadata.len(), 1);
        assert_eq!(body.metadata[0].books[0].name, "1 Nephi");
        assert!(body.playable_verse_count > 0);
        assert!(body.total_verse_count >= body.playable_verse_count);
    }

    #[tokio::test]
    async fn chapter_endpoint_returns_the_requested_chapter() {
        let (app, _state) = app();
        let response = post_json(
            app,
            "/api/chapter",
            ChapterRequest {
                canon: Canon::BookOfMormon,
                book: "Mosiah".to_string(),
                chapter: 2,
            },
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: ChapterResponse = read_json(response).await;
        assert_eq!(body.verses.first().unwrap().verse, 1);
        assert!(body.verses.len() > 20);
    }

    #[tokio::test]
    async fn metadata_endpoint_counts_selected_books_but_keeps_book_choices() {
        let (app, _state) = app();
        let response = post_json(
            app,
            "/api/metadata",
            MetadataRequest {
                difficulty: Difficulty::Normal,
                scope: alma_scope(),
            },
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: MetadataResponse = read_json(response).await;
        assert_eq!(body.metadata.len(), 1);
        assert!(
            body.metadata[0]
                .books
                .iter()
                .any(|book| book.name == "Alma")
        );
        assert!(
            body.metadata[0]
                .books
                .iter()
                .any(|book| book.name == "Helaman")
        );
        assert!(body.playable_verse_count > 0);
    }

    #[tokio::test]
    async fn create_game_returns_prompts_without_answers() {
        let (app, _state) = app();
        let response = post_json(app, "/api/games", test_request()).await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: NewGameResponse = read_json(response).await;
        assert_eq!(body.rounds.len(), 1);
        assert!(!body.game_id.is_empty());
        assert!(!body.rounds[0].text.is_empty());
    }

    #[tokio::test]
    async fn game_snapshot_restores_guess_and_round_progress() {
        let (app, state) = app();
        let response = post_json(
            app.clone(),
            "/api/games",
            NewGameRequest::Random {
                round_count: 2,
                difficulty: Difficulty::Normal,
                scope: GameMode::BookOfMormon.scope(),
            },
        )
        .await;
        let created: NewGameResponse = read_json(response).await;

        let initial: GameSnapshotResponse =
            read_json(get(app.clone(), &format!("/api/games/{}", created.game_id)).await).await;
        assert_eq!(initial.current_round_index, 0);
        assert!(!initial.finished);
        assert!(initial.rounds.iter().all(|round| round.guess.is_none()));

        let answer = state.games.lock().unwrap()[&created.game_id].rounds[0]
            .passage
            .first_reference()
            .unwrap();
        let response = post_json(
            app.clone(),
            &format!("/api/games/{}/guesses", created.game_id),
            GuessRequest {
                round_index: 0,
                canon: answer.canon,
                book: answer.book,
                chapter: answer.chapter,
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let answered: GameSnapshotResponse =
            read_json(get(app.clone(), &format!("/api/games/{}", created.game_id)).await).await;
        assert!(answered.rounds[0].guess.is_some());
        assert_eq!(answered.current_round_index, 0);

        let advanced: AdvanceGameResponse = read_json(
            post_json(
                app.clone(),
                &format!("/api/games/{}/advance", created.game_id),
                (),
            )
            .await,
        )
        .await;
        assert_eq!(advanced.current_round_index, 1);
        assert!(!advanced.finished);

        let restored: GameSnapshotResponse =
            read_json(get(app, &format!("/api/games/{}", created.game_id)).await).await;
        assert_eq!(restored.current_round_index, 1);
        assert!(restored.rounds[0].guess.is_some());
        assert!(restored.rounds[1].guess.is_none());
    }

    #[tokio::test]
    async fn create_game_rejects_excessive_round_counts() {
        let (app, _state) = app();
        let request = NewGameRequest::Random {
            round_count: MAX_GAME_ROUNDS + 1,
            difficulty: Difficulty::Normal,
            scope: GameMode::BookOfMormon.scope(),
        };

        let response = post_json(app, "/api/games", request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn study_game_resolves_only_the_requested_number_of_rounds() {
        let (app, state) = app();
        let set = built_in_study_sets()
            .iter()
            .find(|set| set.passages.len() > 10)
            .unwrap();
        let request = NewGameRequest::Study {
            round_count: 5,
            difficulty: Difficulty::Normal,
            scope: set.resolved_guess_scope(),
            passages: set.passages.clone(),
            prompt_policy: set.prompt_policy,
        };

        let response = post_json(app, "/api/games", request).await;
        let body: NewGameResponse = read_json(response).await;

        assert_eq!(body.rounds.len(), 5);
        assert_eq!(state.games.lock().unwrap()[&body.game_id].rounds.len(), 5);
    }

    #[tokio::test]
    async fn study_game_rejects_an_invalid_unselected_passage() {
        let (app, _state) = app();
        let mut passages = (1..=10)
            .map(|verse| crate::study_sets::StudyPassage {
                canon: Canon::BookOfMormon,
                book: "1 Nephi".to_string(),
                chapter: 1,
                verses: vec![verse],
            })
            .collect::<Vec<_>>();
        passages.push(crate::study_sets::StudyPassage {
            canon: Canon::BookOfMormon,
            book: "1 Nephi".to_string(),
            chapter: 1,
            verses: vec![999],
        });
        let request = NewGameRequest::Study {
            round_count: 1,
            difficulty: Difficulty::Normal,
            scope: GameMode::BookOfMormon.scope(),
            passages,
            prompt_policy: crate::study_sets::PromptPolicy::WholePassage,
        };

        let response = post_json(app, "/api/games", request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_game_uses_selected_book_scope() {
        let (app, state) = app();
        let response = post_json(app.clone(), "/api/games", alma_request()).await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: NewGameResponse = read_json(response).await;
        let answer = state
            .games
            .lock()
            .unwrap()
            .get(&body.game_id)
            .unwrap()
            .rounds[0]
            .passage
            .clone();
        assert_eq!(answer.book, "Alma");
    }

    #[tokio::test]
    async fn create_review_game_uses_each_requested_verse_once() {
        let (app, state) = app();
        let references = [
            crate::scriptures::Reference {
                canon: Canon::BookOfMormon,
                book: "Alma".to_string(),
                chapter: 32,
                verse: 21,
            },
            crate::scriptures::Reference {
                canon: Canon::BookOfMormon,
                book: "Alma".to_string(),
                chapter: 37,
                verse: 6,
            },
        ];
        let passages = references
            .iter()
            .map(crate::study_sets::StudyPassage::single)
            .collect();
        let request = study_request(
            alma_scope(),
            passages,
            crate::study_sets::PromptPolicy::WholePassage,
        );

        let response = post_json(app, "/api/games", request).await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: NewGameResponse = read_json(response).await;
        assert_eq!(body.rounds.len(), references.len());
        assert_eq!(body.playable_verse_count, references.len());

        let games = state.games.lock().unwrap();
        let rounds = &games.get(&body.game_id).unwrap().rounds;
        assert!(references.iter().all(|reference| {
            rounds
                .iter()
                .any(|round| round.passage == crate::study_sets::StudyPassage::single(reference))
        }));
    }

    #[tokio::test]
    async fn guess_endpoint_returns_score_and_chapter_verses() {
        let (app, state) = app();
        let response = post_json(app.clone(), "/api/games", test_request()).await;
        let game: NewGameResponse = read_json(response).await;
        let answer = state
            .games
            .lock()
            .unwrap()
            .get(&game.game_id)
            .unwrap()
            .rounds[0]
            .passage
            .clone();

        let response = post_json(
            app,
            &format!("/api/games/{}/guesses", game.game_id),
            GuessRequest {
                round_index: 0,
                canon: answer.canon,
                book: answer.book.clone(),
                chapter: answer.chapter,
            },
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: GuessResponse = read_json(response).await;
        assert_eq!(body.score.points, crate::scoring::MAX_SCORE);
        assert_eq!(body.answer, answer);
        assert_eq!(body.chapter_verses[0].verse, 1);
        assert!(
            body.chapter_verses
                .iter()
                .any(|verse| answer.contains_verse(verse.verse))
        );
    }

    #[tokio::test]
    async fn guess_endpoint_preserves_a_multi_verse_answer() {
        let (app, _state) = app();
        let passage = crate::study_sets::StudyPassage {
            canon: Canon::BookOfMormon,
            book: "Alma".to_string(),
            chapter: 7,
            verses: vec![11, 12, 13],
        };
        let request = study_request(
            alma_scope(),
            vec![passage.clone()],
            crate::study_sets::PromptPolicy::WholePassage,
        );

        let response = post_json(app.clone(), "/api/games", request).await;
        let game: NewGameResponse = read_json(response).await;
        let response = post_json(
            app,
            &format!("/api/games/{}/guesses", game.game_id),
            GuessRequest {
                round_index: 0,
                canon: passage.canon,
                book: passage.book.clone(),
                chapter: passage.chapter,
            },
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: GuessResponse = read_json(response).await;
        assert_eq!(body.answer, passage);
        assert!(body.answer.contains_verse(11));
        assert!(body.answer.contains_verse(13));
    }

    #[tokio::test]
    async fn automatic_prompt_policy_uses_one_verse_from_long_passages() {
        let (app, _state) = app();
        let passage = crate::study_sets::StudyPassage {
            canon: Canon::BookOfMormon,
            book: "Moroni".to_string(),
            chapter: 7,
            verses: vec![45, 46, 47, 48],
        };
        let request = study_request(
            GameMode::BookOfMormon.scope(),
            vec![passage.clone()],
            crate::study_sets::PromptPolicy::Automatic,
        );

        let response = post_json(app.clone(), "/api/games", request).await;
        let game: NewGameResponse = read_json(response).await;
        let response = post_json(
            app,
            &format!("/api/games/{}/guesses", game.game_id),
            GuessRequest {
                round_index: 0,
                canon: passage.canon,
                book: passage.book.clone(),
                chapter: passage.chapter,
            },
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: GuessResponse = read_json(response).await;
        assert_eq!(body.answer.verses.len(), 1);
        assert_eq!(body.source_passage, passage);
        assert!(body.source_passage.contains_verse(body.answer.verses[0]));
    }

    #[tokio::test]
    async fn expired_game_returns_not_found_on_guess() {
        let (app, state) = app();
        state.games.lock().unwrap().insert(
            "expired".to_string(),
            empty_server_game(SystemTime::UNIX_EPOCH),
        );

        let response = post_json(
            app,
            "/api/games/expired/guesses",
            GuessRequest {
                round_index: 0,
                canon: Canon::BookOfMormon,
                book: "1 Nephi".to_string(),
                chapter: 1,
            },
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
