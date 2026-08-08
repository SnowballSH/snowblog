FROM docker.io/library/rust:1.97-bookworm@sha256:14bc9c5966e7b3a385794b3d5389a8765668342025fbcc7b2e3d2866ac4bd8c3 AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p snowblog

FROM docker.io/library/debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241
ARG SOURCE_COMMIT=unknown
LABEL org.opencontainers.image.source="https://github.com/SnowballSH/snowblog"
LABEL org.opencontainers.image.revision="${SOURCE_COMMIT}"
LABEL org.opencontainers.image.description="Typst blog service"

RUN groupadd --gid 10001 snowblog \
    && useradd --uid 10001 --gid 10001 --create-home --shell /usr/sbin/nologin snowblog \
    && install -d -o snowblog -g snowblog /data

COPY --from=build /src/target/release/snowblog /usr/local/bin/snowblog
COPY vendor/packages /srv/snowblog/vendor/packages

USER snowblog
ENV SNOWBLOG_LISTEN=0.0.0.0:8080 \
    SNOWBLOG_DATABASE=/data/blog.db \
    SNOWBLOG_PACKAGE_ROOT=/srv/snowblog/vendor/packages

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/snowblog"]
CMD ["serve"]
