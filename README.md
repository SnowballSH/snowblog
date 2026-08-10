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

## Metrics

Prometheus metrics are disabled by default. Set `--metrics-listen` or
`SNOWBLOG_METRICS_LISTEN` to a socket address to install the recorder and bind
an independent metrics listener:

```sh
snowblog serve \
  --database blog.db \
  --listen 127.0.0.1:8080 \
  --metrics-listen 127.0.0.1:9090
```

The metrics listener serves `GET /metrics`. It is separate from the
application listener: the application router has no `/metrics` route and
continues to return its normal API 404 there. Omitting the metrics setting
installs no recorder and opens no second listener.

The stable metric families and labels are:

| Family | Type | Labels |
| --- | --- | --- |
| `snowblog_http_requests_total` | counter | normalized route, method, status class |
| `snowblog_http_request_duration_seconds` | histogram | normalized route, method, status class |
| `snowblog_http_requests_in_flight` | gauge | normalized route, method |
| `snowblog_store_operations_total` | counter | fixed operation, `ok` or `error` |
| `snowblog_store_operation_duration_seconds` | histogram | fixed operation |
| `snowblog_sqlite_contention_total` | counter | fixed operation, `busy` or `locked` |
| `snowblog_render_attempts_total` | counter | render operation, outcome |
| `snowblog_render_duration_seconds` | histogram | render operation, `success` or `failure` |
| `snowblog_content_posts` | gauge | `draft`, `published`, or `archived` |
| `snowblog_build_info` | gauge fixed at 1 | service, renderer, and schema versions |

HTTP methods are limited to `get`, `head`, `post`, `put`, `patch`, `delete`,
`options`, and `other`; statuses are limited to `1xx` through `5xx`; unmatched
routes use the literal `unmatched`. Store operations are limited to
`get_post`, `list_posts`, `create_post`, `update_post_meta`, `set_status`,
`delete_post`, `save_translation`, `delete_translation`, `save_asset`,
`delete_asset`, `get_asset`, `get_assets`, and `replace_render`. Render
operations are `preview`, `persisted`, or `rerender`; attempt outcomes are
`success`, `failure`, or `discarded`. Labels never contain request targets,
content values, identifiers, credentials, database statements, or error and
diagnostic text.

The metrics listener has no authentication or transport security of its own.
Bind it to a private interface, or publish it only on host loopback or a
private network protected by an appropriate access-control boundary. Do not
publish it directly to an untrusted network. A container may require a
wildcard bind inside its own network namespace, but the published host socket
must still remain private.

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
