import { useEffect, useState } from "react";

import { type ScoreEntry, fetchScoreboard } from "./api.js";

export function Scoreboard() {
  const [scores, setScores] = useState<ScoreEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    void fetchScoreboard()
      .then((entries) => {
        if (active) {
          setScores(entries);
        }
      })
      .catch((reason: unknown) => {
        if (active) {
          setError(reason instanceof Error ? reason.message : "Could not load scores");
        }
      });

    return () => {
      active = false;
    };
  }, []);

  if (error) {
    return <p role="alert">{error}</p>;
  }

  return (
    <ol aria-label="Scoreboard">
      {scores.map((entry) => (
        <li key={entry.player_id}>
          {entry.player_id}: {entry.score}
        </li>
      ))}
    </ol>
  );
}
