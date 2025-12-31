FROM lukemathwalker/cargo-chef:latest-rust-1.92.0 AS chef

ARG APP_HOME=/app

WORKDIR ${APP_HOME}

RUN apt update && apt install lld clang wait-for-it -y

# Install development cargo tools
RUN rustup component add rustfmt
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo install --version='~0.8' sqlx-cli --no-default-features --features rustls,postgres
RUN cargo binstall bunyan
RUN cargo binstall bacon


FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS runner

# devcontainer dependencies and utils
RUN apt-get update && apt-get install --no-install-recommends -y \
    sudo git nano ssh

# Create devcontainer user and add it to sudoers
RUN groupadd --gid 1000 dev-user \
    && useradd --uid 1000 --gid dev-user --shell /bin/bash --create-home dev-user \
    && echo dev-user ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/dev-user \
    && chmod 0440 /etc/sudoers.d/dev-user

COPY --from=planner ${APP_HOME}/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE=true

COPY compose/production/downloader-rs/entrypoint /entrypoint
RUN sed -i 's/\r$//g' /entrypoint
RUN chmod +x /entrypoint

COPY compose/local/downloader-rs/start /start
RUN sed -i 's/\r$//g' /start
RUN chmod +x /start

ENV APP_ENVIRONMENT=local
CMD ["/entrypoint"]
