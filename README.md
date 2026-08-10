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

## Web frontend

`web/` contains snowblog-web, a SvelteKit SSR frontend for the public API,
built on [Foundation UI](https://github.com/SnowballSH/foundationui). It
serves the post list, post pages with translations, a sitemap, and robots.txt.

```sh
cd web
bun install
SNOWBLOG_API_URL=http://127.0.0.1:8080 bun run dev
```

`bun run build` produces an adapter-node server (`node build`);
`Containerfile.web` packages it. See `web/README.md` for configuration and
the single-origin deployment contract.
