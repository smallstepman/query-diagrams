from dagster import Definitions

from .assets import scoreboard_rollup


defs = Definitions(assets=[scoreboard_rollup])
