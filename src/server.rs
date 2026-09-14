use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::api::{
    CanonMetadata, ChapterVerse, GuessReference, GuessRequest, GuessResponse, MetadataRequest,
    MetadataResponse, NewGameRequest, NewGameResponse, RoundPrompt,
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
    let state = AppState::new(game_ttl)?;

    let app = router_for(state, static_dir);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn router_for(state: AppState, static_dir: impl Into<String>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/metadata", post(metadata))
        .route("/api/games", post(create_game))
        .route("/api/games/{game_id}/guesses", post(submit_guess))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .fallback_service(ServeDir::new(static_dir.into()).append_index_html_on_directories(true))
        .with_state(state)
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("scripguessr=info,tower_http=info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

#[derive(Clone)]
struct AppState {
    library: Arc<ScriptureLibrary>,
    games: Arc<Mutex<BTreeMap<String, ServerGame>>>,
    game_ttl: Duration,
}

impl AppState {
    fn new(game_ttl: Duration) -> Result<Self, serde_json::Error> {
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
            games: Arc::new(Mutex::new(BTreeMap::new())),
            game_ttl,
        })
    }

    fn prune_games(&self) {
        let cutoff = SystemTime::now()
            .checked_sub(self.game_ttl)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        self.games
            .lock()
            .expect("game store poisoned")
            .retain(|_id, game| game.last_seen_at >= cutoff);
    }
}

#[derive(Clone)]
struct ServerGame {
    request: NewGameRequest,
    rounds: Vec<Verse>,
    last_seen_at: SystemTime,
}

async fn healthz() -> &'static str {
    "ok"
}

async fn metadata(
    State(state): State<AppState>,
    Json(request): Json<MetadataRequest>,
) -> Result<Json<MetadataResponse>, ApiError> {
    if request.canons.is_empty() {
        return Err(ApiError::bad_request("At least one canon must be selected"));
    }

    Ok(Json(MetadataResponse {
        difficulty: request.difficulty,
        canons: request.canons.clone(),
        metadata: metadata_for(&state.library, request.difficulty, &request.canons),
        playable_verse_count: playable_verse_count(
            &state.library,
            request.difficulty,
            &request.canons,
        ),
        total_verse_count: total_verse_count(&state.library, &request.canons),
    }))
}

async fn create_game(
    State(state): State<AppState>,
    Json(request): Json<NewGameRequest>,
) -> Result<Json<NewGameResponse>, ApiError> {
    state.prune_games();

    if request.round_count == 0 || request.canons.is_empty() {
        return Err(ApiError::bad_request("Game must include rounds and canons"));
    }

    let playable_count = playable_verse_count(&state.library, request.difficulty, &request.canons);
    if playable_count == 0 {
        return Err(ApiError::bad_request("No playable verses found"));
    }

    let mut rng = SmallRng::from_os_rng();
    let mut rounds = Vec::with_capacity(request.round_count);
    for _round in 0..request.round_count {
        rounds.push(random_verse(
            &state.library,
            &request,
            playable_count,
            &mut rng,
        )?);
    }

    let game_id = rng.random::<u64>().to_string();
    state.games.lock().expect("game store poisoned").insert(
        game_id.clone(),
        ServerGame {
            request: request.clone(),
            rounds: rounds.clone(),
            last_seen_at: SystemTime::now(),
        },
    );

    Ok(Json(NewGameResponse {
        game_id,
        rounds: rounds
            .iter()
            .map(|verse| RoundPrompt {
                text: verse.text.clone(),
            })
            .collect(),
        metadata: metadata_for(&state.library, request.difficulty, &request.canons),
        playable_verse_count: playable_count,
        total_verse_count: total_verse_count(&state.library, &request.canons),
        max_total_score: request.max_total_score(),
    }))
}

async fn submit_guess(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
    Json(request): Json<GuessRequest>,
) -> Result<Json<GuessResponse>, ApiError> {
    state.prune_games();

    let game = {
        let mut games = state.games.lock().expect("game store poisoned");
        let game = games
            .get_mut(&game_id)
            .ok_or_else(|| ApiError::not_found("Game not found"))?;
        game.last_seen_at = SystemTime::now();
        game.clone()
    };
    let answer = game
        .rounds
        .get(request.round_index)
        .ok_or_else(|| ApiError::bad_request("Round not found"))?
        .reference
        .clone();
    let score = state.library.score(
        &game.request.canons,
        &answer,
        request.canon,
        &request.book,
        request.chapter,
    );
    let chapter_verses = state
        .library
        .scriptures(answer.canon)
        .ok_or_else(|| ApiError::not_found("Canon not found"))?
        .verses_for_chapter(&answer)
        .into_iter()
        .map(|verse| ChapterVerse {
            verse: verse.reference.verse,
            text: verse.text,
        })
        .collect();

    Ok(Json(GuessResponse {
        answer,
        guess: GuessReference {
            canon: request.canon,
            book: request.book,
            chapter: request.chapter,
        },
        score,
        chapter_verses,
    }))
}

fn random_verse(
    library: &ScriptureLibrary,
    request: &NewGameRequest,
    playable_count: usize,
    rng: &mut SmallRng,
) -> Result<Verse, ApiError> {
    let mut index = rng.random_range(0..playable_count);

    for canon in &request.canons {
        let scriptures = library
            .scriptures(*canon)
            .ok_or_else(|| ApiError::bad_request("Unknown canon"))?;
        let pool = scriptures.verses_for_difficulty(request.difficulty);

        if index < pool.len() {
            return Ok(pool[index].clone());
        }

        index -= pool.len();
    }

    Err(ApiError::bad_request("No playable verses found"))
}

fn metadata_for(
    library: &ScriptureLibrary,
    difficulty: crate::scriptures::Difficulty,
    canons: &[Canon],
) -> Vec<CanonMetadata> {
    canons
        .iter()
        .filter_map(|canon| {
            let scriptures = library.scriptures(*canon)?;
            Some(CanonMetadata {
                canon: *canon,
                books: scriptures.books.clone(),
                playable_verse_count: scriptures.verse_count_for_difficulty(difficulty),
                total_verse_count: scriptures.total_verse_count(),
            })
        })
        .collect()
}

fn playable_verse_count(
    library: &ScriptureLibrary,
    difficulty: crate::scriptures::Difficulty,
    canons: &[Canon],
) -> usize {
    canons
        .iter()
        .filter_map(|canon| library.scriptures(*canon))
        .map(|scriptures| scriptures.verse_count_for_difficulty(difficulty))
        .sum()
}

fn total_verse_count(library: &ScriptureLibrary, canons: &[Canon]) -> usize {
    canons
        .iter()
        .filter_map(|canon| library.scriptures(*canon))
        .map(Scriptures::total_verse_count)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scriptures::Difficulty;
    use axum::body::{Body, to_bytes};
    use axum::http::{Method, Request, header};
    use serde::de::DeserializeOwned;
    use tower::ServiceExt;

    fn test_request() -> NewGameRequest {
        NewGameRequest {
            round_count: 1,
            difficulty: Difficulty::Normal,
            canons: vec![Canon::BookOfMormon],
        }
    }

    #[test]
    fn pruning_removes_only_expired_games() {
        let state = AppState::new(Duration::from_secs(60)).unwrap();
        state.games.lock().unwrap().insert(
            "expired".to_string(),
            ServerGame {
                request: test_request(),
                rounds: Vec::new(),
                last_seen_at: SystemTime::UNIX_EPOCH,
            },
        );
        state.games.lock().unwrap().insert(
            "active".to_string(),
            ServerGame {
                request: test_request(),
                rounds: Vec::new(),
                last_seen_at: SystemTime::now(),
            },
        );

        state.prune_games();

        let games = state.games.lock().unwrap();
        assert!(!games.contains_key("expired"));
        assert!(games.contains_key("active"));
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
                canons: vec![Canon::BookOfMormon],
            },
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: MetadataResponse = read_json(response).await;
        assert_eq!(body.canons, vec![Canon::BookOfMormon]);
        assert_eq!(body.metadata.len(), 1);
        assert_eq!(body.metadata[0].books[0].name, "1 Nephi");
        assert!(body.playable_verse_count > 0);
        assert!(body.total_verse_count >= body.playable_verse_count);
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
            .reference
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
                .any(|verse| verse.verse == answer.verse)
        );
    }

    #[tokio::test]
    async fn expired_game_returns_not_found_on_guess() {
        let (app, state) = app();
        state.games.lock().unwrap().insert(
            "expired".to_string(),
            ServerGame {
                request: test_request(),
                rounds: Vec::new(),
                last_seen_at: SystemTime::UNIX_EPOCH,
            },
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
