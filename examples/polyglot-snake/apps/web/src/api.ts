export type Direction = "up" | "down" | "left" | "right";

export interface ScoreEntry {
  player_id: string;
  score: number;
}

interface MoveRequest {
  player_id: string;
  direction: Direction;
}

const apiBase = import.meta.env.VITE_API_BASE ?? "http://localhost:3000";

/** @arch edge kind="consumes" to="contract://http/game-api/submit-move" */
export async function submitMove(gameId: string, direction: Direction): Promise<void> {
  const move: MoveRequest = { player_id: "browser-player", direction };
  const response = await fetch(`${apiBase}/v1/games/${encodeURIComponent(gameId)}/moves`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(move),
  });

  if (!response.ok) {
    throw new Error(`submitMove failed with ${response.status}`);
  }
}

/** @arch edge kind="consumes" to="contract://http/game-api/get-scoreboard" */
export async function fetchScoreboard(): Promise<ScoreEntry[]> {
  const response = await fetch(`${apiBase}/v1/scoreboard`);
  if (!response.ok) {
    throw new Error(`fetchScoreboard failed with ${response.status}`);
  }
  return (await response.json()) as ScoreEntry[];
}
