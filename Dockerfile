FROM rust:1

WORKDIR /app

COPY . /app
RUN cargo build --release

CMD ["cargo", "run", "--release"]

