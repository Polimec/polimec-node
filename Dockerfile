# This is an example build stage for the node template. Here we create the binary in a temporary image.

# This is a base image to build substrate nodes
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /build

ARG PROFILE=polimec-parachain-node

COPY . .

RUN cargo build --locked --release -p $PROFILE


# ===== SECOND STAGE ======

FROM docker.io/phusion/baseimage:jammy-1.0.1
LABEL description="Multistage Docker image for Substrate Node Template" \
  image.type="builder" \
  image.authors="you@email.com" \
  image.vendor="Substrate Developer Hub" \
  image.description="Multistage Docker image for Substrate Node Template" \
  image.source="https://github.com/substrate-developer-hub/substrate-node-template" \
  image.documentation="https://github.com/substrate-developer-hub/substrate-node-template"

ARG NODE_TYPE=polimec-parachain-node

COPY --from=builder /build/target/release/$NODE_TYPE /usr/local/bin/node-executable

RUN useradd -m -u 1000 -U -s /bin/sh -d /node node && \
	mkdir -p /node/.local/share/node && \
	chown -R node:node /node/.local && \
	ln -s /node/.local/share/node /data && \
    # unclutter and minimize the attack surface
	rm -rf /usr/bin /usr/sbin && \
    # check if executable works in this container
	/usr/local/bin/node-executable --version

USER node
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

COPY ./dev-specs /node/dev-specs

ENTRYPOINT ["/usr/local/bin/node-executable"]