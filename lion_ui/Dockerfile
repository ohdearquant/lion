FROM rust:1.70 AS builder
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y libssl-dev
RUN cargo build --release -p lion_ui

FROM debian:11-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates
COPY --from=builder /app/target/release/lion_ui /usr/local/bin/lion-ui

EXPOSE 8080
CMD ["lion-ui"]
