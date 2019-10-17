FROM rust:slim

RUN echo "increment for cache busting: 0"

#RUN apt-get update && apt-get install -y build-essential \
#    llvm-3.9-dev libclang-3.9-dev clang-3.9

WORKDIR /app/build

# Hack to cache the dependency builds.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src
RUN echo "fn main() {} // This is a hack" >> src/main.rs
RUN cargo build --release
RUN rm -f src/*.rs

COPY ./src ./src
RUN cargo build --release

RUN cp /app/build/target/release/queerjs_bot /app/queerjs_bot \
 && chmod +x /app/queerjs_bot

RUN rm -rf /app

ENTRYPOINT /app/queerjs_bot
