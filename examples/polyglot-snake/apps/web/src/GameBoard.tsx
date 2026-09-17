import { useState } from "react";

import { type Direction, submitMove } from "./api.js";

interface GameBoardProps {
  gameId: string;
}

export function GameBoard({ gameId }: GameBoardProps) {
  const [status, setStatus] = useState("Choose a direction");

  async function move(direction: Direction) {
    setStatus(`Submitting ${direction}…`);
    try {
      await submitMove(gameId, direction);
      setStatus(`Moved ${direction}`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Move submission failed");
    }
  }

  return (
    <section aria-label="Snake game board">
      <p>{status}</p>
      {(["up", "down", "left", "right"] as const).map((direction) => (
        <button key={direction} type="button" onClick={() => void move(direction)}>
          Move {direction}
        </button>
      ))}
    </section>
  );
}
