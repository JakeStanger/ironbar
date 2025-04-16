FROM rust:latest

COPY .github/scripts/ubuntu_setup.sh /scripts/ubuntu_setup.sh
RUN /scripts/ubuntu_setup.sh

RUN rustup component add clippy