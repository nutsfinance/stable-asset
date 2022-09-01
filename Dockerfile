FROM ubuntu:20.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y \
    build-essential clang git\
    curl cmake

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"
RUN rustup default nightly-2022-05-15 && rustup target add wasm32-unknown-unknown --toolchain nightly-2022-05-15
COPY . /stable-assset
WORKDIR /stable-assset/demo
RUN cargo build
CMD cargo run --bin node -- --dev --ws-external
EXPOSE 9944
