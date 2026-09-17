from __future__ import annotations

from collections import defaultdict
from collections.abc import Iterable
from dataclasses import dataclass


@dataclass(frozen=True)
class ScoreEvent:
    player_id: str
    score_delta: int


@dataclass(frozen=True)
class Score:
    player_id: str
    score: int


def calculate_scores(events: Iterable[ScoreEvent]) -> list[Score]:
    totals: dict[str, int] = defaultdict(int)
    for event in events:
        totals[event.player_id] += event.score_delta

    return [Score(player_id=player_id, score=score) for player_id, score in sorted(totals.items())]
