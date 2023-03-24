FROM rust as builder
COPY . /app
WORKDIR /app
RUN apt update
RUN apt update && apt-get install -y protobuf-compiler
RUN cargo build --release

FROM gcr.io/distroless/cc-debian11 as runtime
WORKDIR /app
COPY --from=builder /app/target/release/social-service .
COPY --from=builder /app/configuration.toml .
ENTRYPOINT [ "./social-service" ]
