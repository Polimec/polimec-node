# This is the build stage for Polkadot. Here we create the binary in a temporary image.
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /polimec
COPY . /polimec
RUN cargo build --locked --release

# This is the 2nd stage: a very small image where we copy the Polkadot binary."
FROM gcr.io/distroless/cc
ARG PACKAGE
COPY --from=builder /polimec/target/release/$PACKAGE /usr/local/bin/polimec

EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/polimec"]