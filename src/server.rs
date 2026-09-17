use std::collections::{BTreeMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
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
        .layer(DefaultBodyLimit::max(MAX_GAME_REQUEST_BYTES))
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
    scope: crate::scriptures::GameScope,
    rounds: Vec<RoundAnswer>,
    last_seen_at: SystemTime,
}

#[derive(Clone)]
struct RoundAnswer {
    passage: crate::study_sets::StudyPassage,
    source_passage: crate::study_sets::StudyPassage,
    text: String,
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

async fn create_game(
    State(state): State<AppState>,
    Json(request): Json<NewGameRequest>,
) -> Result<Json<NewGameResponse>, ApiError> {
    state.prune_games();

    let mut rng = SmallRng::from_os_rng();
    let (difficulty, scope, rounds, playable_count) = match request {
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
            (difficulty, scope, rounds, playable_count)
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
            (difficulty, scope, rounds, playable_count)
        }
    };
    let round_count = rounds.len();

    let game_id = rng.random::<u64>().to_string();
    state.games.lock().expect("game store poisoned").insert(
        game_id.clone(),
        ServerGame {
            scope: scope.clone(),
            rounds: rounds.clone(),
            last_seen_at: SystemTime::now(),
        },
    );

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

async fn submit_guess(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
    Json(request): Json<GuessRequest>,
) -> Result<Json<GuessResponse>, ApiError> {
    state.prune_games();

    let (scope, round) = {
        let mut games = state.games.lock().expect("game store poisoned");
        let game = games
            .get_mut(&game_id)
            .ok_or_else(|| ApiError::not_found("Game not found"))?;
        game.last_seen_at = SystemTime::now();
        let round = game
            .rounds
            .get(request.round_index)
            .cloned()
            .ok_or_else(|| ApiError::bad_request("Round not found"))?;
        (game.scope.clone(), round)
    };
    let answer = round.passage.clone();
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

    Ok(Json(GuessResponse {
        answer,
        source_passage: round.source_passage,
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

    #[test]
    fn pruning_removes_only_expired_games() {
        let state = AppState::new(Duration::from_secs(60)).unwrap();
        state.games.lock().unwrap().insert(
            "expired".to_string(),
            ServerGame {
                scope: GameMode::BookOfMormon.scope(),
                rounds: Vec::new(),
                last_seen_at: SystemTime::UNIX_EPOCH,
            },
        );
        state.games.lock().unwrap().insert(
            "active".to_string(),
            ServerGame {
                scope: GameMode::BookOfMormon.scope(),
                rounds: Vec::new(),
                last_seen_at: SystemTime::now(),
            },
        );

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
            ServerGame {
                scope: GameMode::BookOfMormon.scope(),
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
