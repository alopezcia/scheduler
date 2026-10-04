#!/bin/bash
set -e

# Si no se define JWT_SECRET, se genera uno y se guarda en el volumen para que
# los tokens sigan siendo válidos tras reiniciar el contenedor.
if [ -z "${JWT_SECRET:-}" ]; then
    if [ ! -s /data/jwt_secret ]; then
        head -c 48 /dev/urandom | base64 | tr -d '\n' > /data/jwt_secret
        chmod 600 /data/jwt_secret
    fi
    export JWT_SECRET="$(cat /data/jwt_secret)"
fi

# Si existe una base de datos semilla (docker/seed/calendar.db) y el volumen
# no tiene base todavia, se usa como punto de partida. Nunca pisa datos existentes.
if [ ! -e /data/calendar.db ] && [ -f /seed/calendar.db ]; then
    cp /seed/calendar.db /data/calendar.db
fi

calendar-backend &
BACKEND_PID=$!
nginx -g 'daemon off;' &
NGINX_PID=$!
PIDS="$BACKEND_PID $NGINX_PID"

trap 'kill -TERM $PIDS 2>/dev/null' TERM INT

# El scheduler comparte la base de datos con el backend, que es quien la crea y aplica
# las migraciones antes de abrir su puerto. Se espera a que escuche para arrancarlo.
if [ "${SCHEDULER_ENABLED:-true}" = "true" ]; then
    for _ in $(seq 1 60); do
        (exec 3<>/dev/tcp/127.0.0.1/"${PORT:-4000}") 2>/dev/null && break
        kill -0 "$BACKEND_PID" 2>/dev/null || break
        sleep 1
    done
    scheduler &
    SCHEDULER_PID=$!
    PIDS="$PIDS $SCHEDULER_PID"
fi

# Si cualquiera de los procesos termina, se detiene el contenedor.
wait -n $PIDS
STATUS=$?
kill -TERM $PIDS 2>/dev/null || true
wait
exit $STATUS
