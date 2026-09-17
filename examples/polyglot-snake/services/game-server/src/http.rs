use serde::Deserialize;

use crate::{
    game::{Direction, GameService},
    repository::{ScoreEntry, ScoreboardRepository},
};

#[derive(Clone)]
pub struct AppState {
    game_service: GameService,
    scoreboard_repository: ScoreboardRepository,
}

impl AppState {
    pub fn new(game_service: GameService, scoreboard_repository: ScoreboardRepository) -> Self {
        Self {
            game_service,
            scoreboard_repository,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MoveRequest {
    pub player_id: String,
    pub direction: Direction,
}

/// @arch edge kind="provides" to="contract://http/game-api/submit-move"
pub async fn post_move(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(game_id): axum::extract::Path<String>,
    axum::Json(request): axum::Json<MoveRequest>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    state
        .game_service
        .apply_move(&game_id, &request.player_id, request.direction)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(axum::http::StatusCode::ACCEPTED)
}

/// @arch edge kind="provides" to="contract://http/game-api/get-scoreboard"
pub async fn get_scoreboard(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<axum::Json<Vec<ScoreEntry>>, axum::http::StatusCode> {
    let scores = state
        .scoreboard_repository
        .load()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(axum::Json(scores))
}
