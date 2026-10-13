FROM rust:latest AS rust

FROM ubuntu:26.04

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

COPY --from=rust /usr/local/rustup /usr/local/rustup
COPY --from=rust /usr/local/cargo /usr/local/cargo

COPY .github/scripts/ubuntu_setup.sh /scripts/ubuntu_setup.sh
RUN /scripts/ubuntu_setup.sh

RUN rustup component add clippy
