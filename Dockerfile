FROM ubuntu:20.04 as builder

ENV DEBIAN_FRONTEND=noninteractive
ARG RUST_VERSION=1.51

RUN apt update && \
    apt install curl build-essential -y && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --profile minimal --default-toolchain $RUST_VERSION

ENV PATH="/root/.cargo/bin:${PATH}"

ADD . /build
WORKDIR /build
RUN cargo build --release 

FROM ubuntu:20.04
COPY --from=builder /build/target/release/pokemon-in-shakespeare /
EXPOSE 5000
ENTRYPOINT /pokemon-in-shakespeare