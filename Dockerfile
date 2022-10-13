FROM rust:1.64.0-buster as builder

ARG COMPONENT

WORKDIR /negy

RUN apt update -y && apt install libssl-dev pkg-config ca-certificates -y

COPY . .

RUN cargo build -p negy-${COMPONENT} --release

FROM debian:buster-slim

ARG COMPONENT
ENV COMPONENT=${COMPONENT}

RUN apt update -y && apt install libssl-dev pkg-config ca-certificates -y

COPY --from=builder /negy/target/release/negy-${COMPONENT} /usr/bin/negy

EXPOSE 3000

ENTRYPOINT ["negy"]
