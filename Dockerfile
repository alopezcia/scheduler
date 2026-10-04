# ---------- Etapa 1: compilar el frontend ----------
FROM node:18-alpine AS frontend
WORKDIR /app

COPY package.json yarn.lock ./
RUN yarn install --frozen-lockfile

COPY index.html vite.config.js ./
COPY src ./src

# Las variables de entorno del proceso tienen prioridad sobre los ficheros .env
# de Vite. El frontend llama a /api en el mismo origen (nginx hace de proxy).
ENV VITE_MODE=prod \
    VITE_API_URL=/api
RUN yarn build

# ---------- Etapa 2: runtime (nginx + backend Rust + scheduler Rust) ----------
# El binario se compiló en Debian 11 (glibc 2.31); bookworm es compatible.
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends nginx ca-certificates \
    && rm -rf /var/lib/apt/lists/* /etc/nginx/sites-enabled/default

# Backend y scheduler: ejecutables Linux compilados previamente (ver README, sección Docker)
COPY backend/target/x86_64-unknown-linux-gnu/release/calendar-backend /usr/local/bin/calendar-backend
COPY scheduler/target/x86_64-unknown-linux-gnu/release/scheduler /usr/local/bin/scheduler
RUN chmod +x /usr/local/bin/calendar-backend /usr/local/bin/scheduler

# Frontend estático
COPY --from=frontend /app/dist /usr/share/nginx/html
COPY docker/nginx.conf /etc/nginx/conf.d/app.conf
COPY docker/seed/ /seed/
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh && mkdir -p /data

ENV DATABASE_URL=sqlite:///data/calendar.db \
    PORT=4000 \
    JWT_EXPIRES_SECONDS=86400 \
    ALLOWED_ORIGIN=http://localhost:8080 \
    SCHEDULER_ENABLED=true \
    POLL_INTERVAL_SECONDS=5 \
    MAX_LATE_SECONDS=300

ARG VERSION=0.2.0
LABEL org.opencontainers.image.version="${VERSION}"

VOLUME /data
EXPOSE 80

ENTRYPOINT ["/entrypoint.sh"]
