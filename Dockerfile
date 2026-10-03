# syntax=docker/dockerfile:1
# The images the `just` recipes run in: `rust` builds and tests, `fmt` formats.
# Versions live in mise/; the base image tag pins mise itself.
FROM ghcr.io/jdx/mise:2026.10.0-debian AS base

ENV CARGO_HOME=/opt/cargo
ENV RUSTUP_HOME=/opt/rustup
ENV MISE_TRUSTED_CONFIG_PATHS=/workspace
ENV MISE_YES=1
# Tools are baked at build time; a run never installs anything.
ENV MISE_EXEC_AUTO_INSTALL=false
# The Rust CDN is sometimes unstable. rustup times out a stalled read and
# retries with resume; allow it more attempts than the default three.
ENV RUSTUP_MAX_RETRIES=10

WORKDIR /workspace

# Downloaded crates live in the mounted repository so they survive between runs.
RUN mkdir -p /opt/cargo /workspace/.cache/cargo/git /workspace/.cache/cargo/registry \
    && ln -s /workspace/.cache/cargo/git /opt/cargo/git \
    && ln -s /workspace/.cache/cargo/registry /opt/cargo/registry

FROM base AS rust

# Only a linker: no dependency builds C code.
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends gcc libc6-dev \
    && rm -rf /var/lib/apt/lists/*

COPY mise/config.toml mise/config.toml

RUN mise install \
    && chmod -R a+rwX /opt/cargo /opt/rustup /mise \
    && mise exec -- cargo clippy --version \
    && mise exec -- sh -c 'test -d "$(rustc --print sysroot)/lib/rustlib/src/rust/library"'

FROM base AS fmt

ENV MISE_ENV=fmt

COPY mise/config.toml mise/config.fmt.toml mise/

RUN mise install \
    && chmod -R a+rwX /opt/cargo /opt/rustup /mise \
    && mise exec -- cargo fmt --version \
    && mise exec -- node --version

# Runtime settings, after the installs so changing them reinstalls nothing.
ENV NPM_CONFIG_CACHE=/workspace/.cache/npm
ENV NPM_CONFIG_UPDATE_NOTIFIER=false
