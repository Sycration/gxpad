# sgweb in a container: a static build of the server, which embeds the whole web app and the sggeo Wasm
# module, in an image with nothing else in it. The build context is the root of the repository:
#
#   docker build -f sgweb/Dockerfile -t sgweb .
#   docker run --init -p 8080:8080 sgweb
#
# or `docker compose -f sgweb/compose.yaml up` (see sgweb/README.md).

# ---------------------------------------------------------------- build
FROM docker.io/library/rust:1.100-slim-bookworm AS build

# the version of wasm-bindgen-cli must be the version of the wasm-bindgen crate in Cargo.lock
ARG WASM_BINDGEN_VERSION=0.2.130

RUN rustup target add wasm32-unknown-unknown "$(uname -m)-unknown-linux-musl"
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install --locked wasm-bindgen-cli --version "$WASM_BINDGEN_VERSION"

WORKDIR /src
COPY . .

# sgweb's build.rs builds sggeo for wasm32 and runs wasm-bindgen on it; the server links statically against
# musl, so the image needs no C library
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    target="$(uname -m)-unknown-linux-musl" \
    && cargo build --release --locked -p sgweb --target "$target" \
    && install -D "target/$target/release/sgweb" /out/sgweb

# ---------------------------------------------------------------- run
FROM scratch

COPY --from=build /out/sgweb /sgweb

# nobody: the server needs no privileges and writes nothing
USER 65534:65534
EXPOSE 8080
ENTRYPOINT ["/sgweb", "--host", "0.0.0.0", "--port", "8080"]
