# web-admin

A SvelteKit backend-for-frontend (BFF) that gives operators a browser UI for
managing snowblog content: creating, editing, previewing, publishing, and
unpublishing Typst posts against the existing snowblog admin API. Browser
JavaScript never talks to the admin API directly — every request to
snowblog, including the bearer token that authorizes it, is made from server
routes running in this app.

The app trusts a reverse proxy in front of it to authenticate the operator
and forward their identity in a header; it never renders anything, and never
proxies any admin API call, without a recognized identity.

## Configuration

The app fails closed: it refuses to start unless every required variable is
present and valid.

| Variable | Meaning | Behavior when missing |
|---|---|---|
| `SNOWBLOG_API_URL` | Origin of the snowblog admin API this app calls | refuse to start |
| `SNOWBLOG_ADMIN_TOKEN_FILE` | Path to a file holding the bearer token used to authenticate to the admin API | refuse to start |
| `ADMIN_IDENTITY_HEADER` | Name of the request header carrying the authenticated operator's username | defaults to `Remote-User` |
| `ADMIN_ALLOWED_USERS` | Comma-separated allowlist of usernames permitted to use the dashboard | refuse to start |
| `ORIGIN` | Public origin of this app, used for SvelteKit's built-in origin checking on form submissions | refuse to start |
| `SNOWBLOG_ADMIN_METRICS_LISTEN` | Address the Prometheus exposition endpoint listens on | metrics stay disabled |

There is no partial startup: without a usable identity header configuration
and a readable, non-empty token file, the app does not serve requests.

## Development

```sh
bun install
SNOWBLOG_API_URL=http://127.0.0.1:8080 \
SNOWBLOG_ADMIN_TOKEN_FILE=/path/to/token \
ADMIN_ALLOWED_USERS=dev \
ORIGIN=http://localhost:5173 \
bun run dev
```

In development, the identity header this app trusts (`Remote-User` unless
`ADMIN_IDENTITY_HEADER` is set) is not injected by anything running
locally — inject it yourself with any header-modifying HTTP client (a
browser extension, a local reverse proxy, or `curl -H`) so the app can
authorize requests as `dev`.

Other useful scripts: `bun run build`, `bun run check`, `bun run lint`,
`bun run format`, `bun run test`.

## Genericity

This app's code never names a specific deployment: no reverse proxy,
identity provider, hosting platform, or hostname. Every fact about where and
how it runs — the API it talks to, who may use it, where it is reachable —
arrives as configuration. It is deployable standalone, wherever a caller can
supply the six variables above and put a proxy in front of it that sets the
identity header after authenticating the operator.
