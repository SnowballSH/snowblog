FROM docker.io/library/rust:1.98-bookworm@sha256:82150a52ec202c1b14d7817e14516c392bb7f5cfebd88f1ed531cb37ebd39922 AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p snowblog

FROM docker.io/library/debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171
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
