# snowblog

A self-contained blog service for posts written in [Typst](https://typst.app).
Sources, translations, and assets live in SQLite; every write renders the post
to HTML with the embedded Typst compiler; a versioned HTTP JSON API serves
published posts publicly and gates all mutations behind a bearer token.

## Build

```sh
cargo build --release
```

## Run

```sh
snowblog db migrate --database blog.db
snowblog serve --database blog.db --listen 127.0.0.1:8080
snowblog import --database blog.db --dir path/to/typ-files
snowblog rerender --database blog.db --scope stale
```

Configuration comes from flags or `SNOWBLOG_*` environment variables:
database path, listen address, admin token file (admin API is disabled
without it), Typst package root, extra font directories, and render bounds.

See `docs/architecture.md` for the rendering pipeline, invariants, and API.
