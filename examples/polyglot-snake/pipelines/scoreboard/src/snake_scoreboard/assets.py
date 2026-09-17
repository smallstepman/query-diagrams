from __future__ import annotations

import os

import psycopg
from dagster import asset

from .compute import Score, ScoreEvent, calculate_scores


@asset
def scoreboard_rollup() -> list[Score]:
    """
    @arch edge kind="reads" to="data://postgres/snake/public/score_events"
    @arch edge kind="writes" to="data://postgres/snake/public/scoreboard"
    """
    database_url = os.environ["DATABASE_URL"]
    with psycopg.connect(database_url) as connection:
        with connection.cursor() as cursor:
            cursor.execute(
                "SELECT player_id, score_delta FROM public.score_events ORDER BY occurred_at ASC"
            )
            events = [ScoreEvent(player_id=row[0], score_delta=row[1]) for row in cursor.fetchall()]
            scores = calculate_scores(events)
            cursor.executemany(
                """
                INSERT INTO public.scoreboard (player_id, score, calculated_at)
                VALUES (%s, %s, now())
                ON CONFLICT (player_id) DO UPDATE
                SET score = EXCLUDED.score, calculated_at = EXCLUDED.calculated_at
                """,
                [(score.player_id, score.score) for score in scores],
            )

    return scores
