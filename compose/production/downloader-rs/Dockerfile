FROM lukemathwalker/cargo-chef:latest-rust-1.92.0 AS chef

ARG APP_HOME=/app

RUN apt update && apt install lld clang -y

FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner ${APP_HOME}/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE=true
# Build our project
RUN cargo build --release --bin downloader

FROM debian:bookworm-slim AS runtime
WORKDIR ${APP_HOME}

RUN addgroup --system downloader \
    && adduser --system --ingroup downloader downloader

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates wait-for-it\
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder --chown=downloader:downloader ${APP_HOME}/target/release/downloader downloader
COPY configuration configuration

COPY --from=builder --chown=downloader:downloader ${APP_HOME}/compose/production/downloader-rs/entrypoint entrypoint
RUN sed -i 's/\r$//g' entrypoint
RUN chmod +x entrypoint

COPY --from=builder --chown=downloader:downloader ${APP_HOME}/compose/production/downloader-rs/start start
RUN sed -i 's/\r$//g' start
RUN chmod +x start

ENV APP_ENVIRONMENT=production
ENTRYPOINT ["./entrypoint"]
