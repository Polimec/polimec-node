# This is the build stage for Polkadot. Here we create the binary in a temporary image.
FROM docker.io/paritytech/ci-linux:production as builder
ARG PACKAGE
WORKDIR /polimec
COPY . /polimec
RUN cargo build --locked --profile production -p $PACKAGE 

# This is the 2nd stage: a very small image where we copy the Polkadot binary."
FROM debian:bookworm-slim
ARG PACKAGE
COPY --from=builder /polimec/target/production/$PACKAGE /usr/local/bin/polimec

# 30333 for parachain p2p
# 30334 for relaychain p2p
# 9944 for Websocket & RPC call
# 9615 for Prometheus (metrics)
EXPOSE 30333 30334 9944 9615

VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/polimec"]