mod game;
mod http;
mod repository;

use std::env;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

use crate::{
    game::GameService,
    http::{AppState, get_scoreboard, post_move},
    repository::{ScoreEventRepository, ScoreboardRepository},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/snake".to_owned());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&database_url)?;

    let state = AppState::new(
        GameService::new(ScoreEventRepository::new(pool.clone())),
        ScoreboardRepository::new(pool),
    );
    let app = Router::new()
        .route("/v1/games/{game_id}/moves", post(post_move))
        .route("/v1/scoreboard", get(get_scoreboard))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
