FROM rust:latest AS builder

WORKDIR /pipe_robot

COPY . /pipe_robot

RUN cargo build --release

EXPOSE 5555

ENV RUST_LOG=info

CMD [ "cargo", "run", "--release" ]
