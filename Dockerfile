FROM rust:slim-buster as build

WORKDIR /app

RUN echo "increment for cache busting: 0"

RUN mkdir src
RUN echo "fn main() {} // This is a hack" >> src/main.rs

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy over real source
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/queerjs*
RUN cargo build --release

FROM debian:buster-slim

RUN apt-get update; \
    apt-get install -y --no-install-recommends \
      ca-certificates; \
    rm -rf /var/lib/apt/lists/*;

WORKDIR /app

COPY --from=build /app/target/release/queerjs_bot ./
RUN chmod +x ./queerjs_bot

CMD ./queerjs_bot
