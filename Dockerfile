FROM rust:1.71.1 as builder
COPY . /app
WORKDIR /app
RUN apt update && apt-get install -y protobuf-compiler
RUN cargo build --release

FROM gcr.io/distroless/cc-debian11 as runtime
WORKDIR /app
COPY --from=builder /app/target/release/social-service .
COPY --from=builder /app/configuration.toml .
EXPOSE 5000
EXPOSE 8085
ENTRYPOINT [ "./social-service" ]
