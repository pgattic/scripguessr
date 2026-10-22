# Sourced by Nix entry points. Requires PostgreSQL on PATH.
set -euo pipefail
export PGHOST="${PGHOST:-$PWD/.nix/postgres/socket}"
export PGPORT="${PGPORT:-55437}"
quiz_pgdata="${SCRIPGUESSR_PGDATA:-$PWD/.nix/postgres/data}"
mkdir -p "$PGHOST"
if [ ! -s "$quiz_pgdata/PG_VERSION" ]; then
  initdb -D "$quiz_pgdata" --auth=trust --no-locale --encoding=UTF8 >/dev/null
fi
quiz_started_postgres=false
if ! pg_ctl -D "$quiz_pgdata" status >/dev/null 2>&1; then
  pg_ctl -D "$quiz_pgdata" -l "$quiz_pgdata/server.log" -o "-k $PGHOST -p $PGPORT -h ''" -w start
  quiz_started_postgres=true
fi
quiz_cleanup() {
  if [ "$quiz_started_postgres" = true ]; then
    pg_ctl -D "$quiz_pgdata" -m fast -w stop
  fi
}
trap quiz_cleanup EXIT
