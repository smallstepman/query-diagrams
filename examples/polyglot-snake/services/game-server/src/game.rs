use serde::Deserialize;

use crate::repository::ScoreEventRepository;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Clone)]
pub struct GameService {
    score_events: ScoreEventRepository,
}

impl GameService {
    pub fn new(score_events: ScoreEventRepository) -> Self {
        Self { score_events }
    }

    pub async fn apply_move(
        &self,
        game_id: &str,
        player_id: &str,
        direction: Direction,
    ) -> Result<(), sqlx::Error> {
        self.score_events
            .append_move(game_id, player_id, direction.as_str())
            .await
    }
}
