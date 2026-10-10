FROM docker.io/library/rust:1.99-bookworm@sha256:114c7a4425406451c2866b6aafe69fe29b1b298832db1277d411ac73c82d04d6 AS build
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
