# syntax=docker/dockerfile:1

ARG PLATFORM=linux/amd64

FROM --platform=${PLATFORM} docker.io/paritytech/ci-unified:latest AS builder

WORKDIR /polkadot
COPY . /polkadot

RUN cargo fetch
RUN cargo build --locked --release

FROM --platform=${PLATFORM} docker.io/parity/base-bin:latest

COPY --from=builder /polkadot/target/release/solochain-starter-node /usr/local/bin

USER root
RUN useradd -m -u 1001 -U -s /bin/sh -d /polkadot polkadot && \
	mkdir -p /data /polkadot/.local/share && \
	chown -R polkadot:polkadot /data && \
	ln -s /data /polkadot/.local/share/polkadot && \
# unclutter and minimize the attack surface
	rm -rf /usr/bin /usr/sbin && \
# check if executable works in this container
	/usr/local/bin/solochain-starter-node --version

USER polkadot

EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/solochain-starter-node"]
