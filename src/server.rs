use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

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

pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(8087);
    let static_dir =
        std::env::var("SCRIPGUESSR_STATIC_DIR").unwrap_or_else(|_| "dist/public".to_string());
    let state = AppState::new()?;

    let app = Router::new()
        .route("/api/metadata", post(metadata))
        .route("/api/games", post(create_game))
        .route("/api/games/{game_id}/guesses", post(submit_guess))
        .layer(CorsLayer::permissive())
        .fallback_service(ServeDir::new(static_dir).append_index_html_on_directories(true))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    library: Arc<ScriptureLibrary>,
    games: Arc<Mutex<BTreeMap<String, ServerGame>>>,
}

impl AppState {
    fn new() -> Result<Self, serde_json::Error> {
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
        })
    }
}

#[derive(Clone)]
struct ServerGame {
    request: NewGameRequest,
    rounds: Vec<Verse>,
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
    let game = state
        .games
        .lock()
        .expect("game store poisoned")
        .get(&game_id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("Game not found"))?;
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
