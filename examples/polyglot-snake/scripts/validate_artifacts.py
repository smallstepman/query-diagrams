#!/usr/bin/env python3
"""Validate the committed, manually curated example artifacts with stdlib only."""

from __future__ import annotations

import json
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError as error:  # Python 3.10 does not include tomllib.
    raise SystemExit("Python 3.11+ is required to validate architecture.toml") from error


def fail(message: str) -> None:
    raise SystemExit(f"artifact validation failed: {message}")


def validate_raw_facts(path: Path) -> None:
    seen: set[str] = set()
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line:
            fail(f"{path}:{line_number}: blank lines are not JSON Lines records")
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            fail(f"{path}:{line_number}: {error.msg}")
        if record.get("record") not in {"node", "edge"}:
            fail(f"{path}:{line_number}: record must be node or edge")
        identifier = record.get("id")
        if not isinstance(identifier, str) or not identifier:
            fail(f"{path}:{line_number}: record needs a nonempty id")
        if identifier in seen:
            fail(f"{path}:{line_number}: duplicate id {identifier!r}")
        seen.add(identifier)
        if record["record"] == "edge" and not all(
            isinstance(record.get(field), str) and record[field]
            for field in ("from", "to", "kind")
        ):
            fail(f"{path}:{line_number}: edge needs kind, from, and to")
        if not isinstance(record.get("provenance"), dict):
            fail(f"{path}:{line_number}: record needs provenance")


def validate_graph(path: Path) -> None:
    try:
        graph = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        fail(f"{path}: {error.msg}")

    if graph.get("schema_version") != "architecture-graph/v1":
        fail(f"{path}: unexpected schema_version")
    nodes = graph.get("nodes")
    edges = graph.get("edges")
    if not isinstance(nodes, list) or not isinstance(edges, list):
        fail(f"{path}: nodes and edges must be arrays")

    node_ids = {node.get("id") for node in nodes if isinstance(node, dict)}
    if len(node_ids) != len(nodes) or None in node_ids:
        fail(f"{path}: nodes need unique IDs")

    components = {
        node["id"]: node.get("component")
        for node in nodes
        if isinstance(node, dict) and node.get("kind") == "code.symbol"
    }
    for edge in edges:
        if not isinstance(edge, dict):
            fail(f"{path}: edge is not an object")
        source = edge.get("from")
        target = edge.get("to")
        if source not in node_ids or target not in node_ids:
            fail(f"{path}: edge {edge.get('id')!r} has an unknown endpoint")
        if source in components and target in components and components[source] != components[target]:
            fail(f"{path}: direct cross-component code edge {edge.get('id')!r} is forbidden")


def main() -> None:
    root = Path(sys.argv[1]).resolve()
    config = root / "architecture.toml"
    raw_facts = root / "expected" / "raw-facts.jsonl"
    graph = root / "expected" / "architecture.graph.json"

    try:
        parsed_config = tomllib.loads(config.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as error:
        fail(f"{config}: {error}")
    if parsed_config.get("workspace", {}).get("id") != "workspace://polyglot-snake":
        fail(f"{config}: workspace.id is missing")
    if len(parsed_config.get("projects", [])) != 3:
        fail(f"{config}: expected exactly three language projects")

    validate_raw_facts(raw_facts)
    validate_graph(graph)
    print("architecture.toml, raw facts, and canonical graph are valid")


if __name__ == "__main__":
    main()
