#!/usr/bin/env bash
set -euo pipefail
image="${1:-binbin:local}"
container="binbin-smoke-$RANDOM"
volume="binbin-smoke-$RANDOM"
cleanup() {
  if [ "$?" -ne 0 ]; then docker logs "$container" 2>&1 || true; fi
  docker rm -f "$container" >/dev/null 2>&1 || true
  docker volume rm "$volume" >/dev/null 2>&1 || true
}
trap cleanup EXIT
probe() {
  local port
  port="$(docker port "$container" 8080/tcp | cut -d: -f2)"
  for attempt in $(seq 1 30); do
    if curl -fsS "http://127.0.0.1:$port/healthz" >/dev/null 2>&1; then break; fi
    sleep 1
  done
  curl -fsS "http://127.0.0.1:$port/healthz" >/dev/null
}
start_container() {
  docker run -d --name "$container" -p 127.0.0.1::8080 -v "$volume:/data" "$image" >/dev/null
  probe
}
docker volume create "$volume" >/dev/null
start_container
docker exec "$container" sh -c 'test -s /data/binbin.sqlite && test "$(id -u)" != 0 && printf persisted > /data/smoke-marker'
docker restart "$container" >/dev/null
probe
docker exec "$container" sh -c 'test -s /data/binbin.sqlite && test "$(cat /data/smoke-marker)" = persisted'
# A replacement container must recover the same named volume, not its old writable layer.
docker rm -f "$container" >/dev/null
start_container
docker exec "$container" sh -c 'test -s /data/binbin.sqlite && test "$(cat /data/smoke-marker)" = persisted'
echo 'Docker startup, non-root execution, restart, and volume persistence passed.'
