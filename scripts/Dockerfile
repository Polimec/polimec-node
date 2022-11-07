# Stage 1

FROM rust:slim as builder
WORKDIR /app

RUN rustup default nightly-2022-11-06 && \
    rustup target add wasm32-unknown-unknown --toolchain nightly-2022-11-06

RUN apt-get update && \
    apt-get install -y \
     cmake \
     pkg-config \
     libssl-dev \
     git \
     clang \
     libclang-dev

ARG GIT_COMMIT=
ENV GIT_COMMIT=$GIT_COMMIT
ARG BUILD_ARGS=build-polimec-release
ARG PROFILE=production

COPY . .

RUN make $BUILD_ARGS

# Stage 2

FROM phusion/baseimage:jammy-1.0.1
LABEL maintainer="info@polimec.org"

ARG PROFILE

RUN useradd -m -u 1000 -U -s /bin/sh -d /polimec polimec

COPY --from=builder /app/target/$PROFILE/polimec /usr/local/bin

# checks
RUN ldd /usr/local/bin/polimec && \
    /usr/local/bin/polimec --version

# Shrinking
RUN rm -rf /usr/lib/python* && \
    rm -rf /usr/sbin /usr/share/man

USER polimec
EXPOSE 30333 9933 9944

RUN mkdir /polimec/data

VOLUME ["/polimec/data"]

ENTRYPOINT ["/usr/local/bin/polimec"]