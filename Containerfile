# This is the build stage for Polkadot. Here we create the binary in a temporary image.
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /polimec
COPY . /polimec
RUN cargo build --locked --release

# This is the 2nd stage: a very small image where we copy the Polkadot binary."
FROM docker.io/library/ubuntu:20.04
ARG PACKAGE
COPY --from=builder /polimec/target/release/$PACKAGE /usr/local/bin/polimec

RUN useradd -m -u 1000 -U -s /bin/sh -d /polimec polimec && \
	mkdir -p /data /polimec/.local/share && \
	chown -R polimec:polimec /data && \
	ln -s /data /polimec/.local/share/polimec && \
# unclutter and minimize the attack surface
	rm -rf /usr/bin /usr/sbin && \
# check if executable works in this container
	/usr/local/bin/polimec --version

USER polimec

EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/polimec"]