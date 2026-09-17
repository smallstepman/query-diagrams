-- @arch node id="data://postgres/snake/public/score_events" kind="postgres.table" label="public.score_events"
CREATE TABLE public.score_events (
    event_id BIGSERIAL PRIMARY KEY,
    game_id TEXT NOT NULL,
    player_id TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    score_delta INTEGER NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- @arch node id="data://postgres/snake/public/scoreboard" kind="postgres.table" label="public.scoreboard"
CREATE TABLE public.scoreboard (
    player_id TEXT PRIMARY KEY,
    score INTEGER NOT NULL,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
