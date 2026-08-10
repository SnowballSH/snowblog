# snowblog-web

A SvelteKit SSR frontend for the snowblog public API, styled with
[Foundation UI](https://github.com/SnowballSH/foundationui). Pages are
rendered on the server per request, so published posts appear immediately
and crawlers see full HTML.

## Configuration

| Variable                  | Required      | Default         | Purpose                                                                                     |
| ------------------------- | ------------- | --------------- | ------------------------------------------------------------------------------------------- |
| `SNOWBLOG_API_URL`        | yes           | —               | Base URL of the snowblog API, used server-side.                                             |
| `ORIGIN`                  | in production | —               | Public origin (`https://blog.example.com`), used by adapter-node and for canonical/OG URLs. |
| `PORT`                    | no            | `3000`          | Listen port of the node server.                                                             |
| `PUBLIC_SITE_NAME`        | no            | `Blogs`         | Site name in the hero and brand lockup.                                                     |
| `PUBLIC_SITE_AUTHOR`      | no            | `SnowballSH`    | Brand prefix in the header; set empty to show only the site name.                           |
| `PUBLIC_SITE_DESCRIPTION` | no            | portfolio blurb | Hero introduction and meta description.                                                     |
| `PUBLIC_FOOTER_TEXT`      | no            | site name       | Footer text.                                                                                |

## Single-origin deployment contract

Rendered post HTML references assets by root-relative URLs
(`/api/v1/posts/{slug}/assets/…`, the server's default
`--asset-url-prefix`). Serve the app and the API under one origin by
routing the public API allowlist (`/api/v1/health`, `/api/v1/posts`,
`/api/v1/posts/*`) to the snowblog server and everything else to this app.
Alternatively, run snowblog with an absolute `--asset-url-prefix` and skip
the shared origin. Never route `/api/v1/admin` through the public origin.

In development, `bun run dev` proxies `/api/v1` to `SNOWBLOG_API_URL` for
you.

## Commands

```sh
bun install
bun run dev        # dev server (set SNOWBLOG_API_URL)
bun run lint       # prettier + eslint
bun run check      # svelte-check
bun run test       # vitest
bun run build      # adapter-node output in build/
bun run smoke      # boots the build against a fixture API and asserts pages
bun run icons      # regenerate favicons from assets/mascot.png
```
