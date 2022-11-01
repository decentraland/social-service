FROM alpine:3.15 as runtime
EXPOSE 8080

FROM rust:alpine3.15 AS chef
RUN apk add musl-dev --no-cache && \
    cargo install cargo-chef
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare

FROM chef AS builder
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook
COPY . .
RUN cargo build --release

FROM runtime AS runner
WORKDIR /app
COPY --from=builder /src/target/release/social-service .
COPY --from=builder /src/configuration.toml .
ENTRYPOINT [ "./social-service" ]
