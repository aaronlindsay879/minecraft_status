FROM rust:1.72 AS builder
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder ./target/release/minecraft_status ./target/release/minecraft_status
CMD ["/target/release/minecraft_status"]
