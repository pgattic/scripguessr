# Sourced by Nix entry points. Requires PostgreSQL on PATH.
set -euo pipefail
export PGHOST="${PGHOST:-$PWD/.nix/postgres/socket}"
export PGPORT="${PGPORT:-55437}"
game_pgdata="${SCRIPGUESSR_PGDATA:-$PWD/.nix/postgres/data}"
mkdir -p "$PGHOST"
if [ ! -s "$game_pgdata/PG_VERSION" ]; then
  initdb -D "$game_pgdata" --auth=trust --no-locale --encoding=UTF8 >/dev/null
fi
game_started_postgres=false
if ! pg_ctl -D "$game_pgdata" status >/dev/null 2>&1; then
  pg_ctl -D "$game_pgdata" -l "$game_pgdata/server.log" -o "-k $PGHOST -p $PGPORT -h ''" -w start
  game_started_postgres=true
fi
game_cleanup() {
  if [ "$game_started_postgres" = true ]; then
    pg_ctl -D "$game_pgdata" -m fast -w stop
  fi
}
trap game_cleanup EXIT
