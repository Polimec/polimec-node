FROM docker.io/paritytech/ci-linux:production AS chef 
RUN cargo install cargo-chef 
WORKDIR /polimec

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json 

FROM chef AS builder
ARG PACKAGE
COPY --from=planner /polimec/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook  -p $PACKAGE --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release -p $PACKAGE

# We do not need the Rust toolchain to run the binary!
FROM debian:buster-slim AS runtime
ARG PACKAGE
WORKDIR /polimec
COPY --from=builder /polimec/target/release/$PACKAGE /usr/local/bin/polimec-collator

RUN find /var/lib/apt/lists/ -type f -not -name lock -delete; \
	useradd -m -u 1000 -U -s /bin/sh -d /polimec-collator polimec-collator && \
	mkdir -p /data /polimec-collator/.local/share && \
	chown -R polimec-collator:polimec-collator /data && \
	ln -s /data /polimec-collator/.local/share/polkadot

USER polimec-collator
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

COPY ./dev-specs /node/dev-specs

ENTRYPOINT ["/usr/local/bin/polimec-collator"]