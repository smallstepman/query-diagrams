use serde::Serialize;
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct ScoreEventRepository {
    pool: PgPool,
}

impl ScoreEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// @arch edge kind="writes" to="data://postgres/snake/public/score_events"
    pub async fn append_move(
        &self,
        game_id: &str,
        player_id: &str,
        direction: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO public.score_events (game_id, player_id, event_kind, score_delta) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(game_id)
        .bind(player_id)
        .bind(direction)
        .bind(0_i32)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct ScoreboardRepository {
    pool: PgPool,
}

#[derive(Debug, Serialize)]
pub struct ScoreEntry {
    pub player_id: String,
    pub score: i32,
}

impl ScoreboardRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// @arch edge kind="reads" to="data://postgres/snake/public/scoreboard"
    pub async fn load(&self) -> Result<Vec<ScoreEntry>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT player_id, score FROM public.scoreboard ORDER BY score DESC, player_id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(ScoreEntry {
                    player_id: row.try_get("player_id")?,
                    score: row.try_get("score")?,
                })
            })
            .collect()
    }
}
