# Agent instructions

snowblog is a standalone blog service: Typst sources stored in SQLite,
rendered to HTML on write by the embedded Typst compiler, served over a
versioned HTTP JSON API (public reads, bearer-token admin writes). It is
generic infrastructure — it must never reference any particular deployment,
reverse proxy, or identity provider.

Before every commit: `cargo fmt --all && cargo clippy --workspace
--all-targets -- -D warnings && cargo test --workspace`.

## Code and Technology Guidelines

- Prefer well-known libraries over lesser-known Github repos for dependencies.
- Prefer well-maintained open-source projects
- Code should be self-documenting. That is, you should not need to, and you should not, write comments explaining your code. Only do so for very non-trivial logic for certain lines. Any complicated overall design choice or logic should be documented in a markdown file instead.
- Prefer the latest version of all technologies (frameworks, languages, etc.).
- Use modern language features and practices.
- Format, lint, and test your code.
- Use good software engineering patterns, including OOP, abstractions, strict typing and contracts, write & review contracts/interfaces before implementation, concise functional programming whenever needed.
- Avoid "hacky" one-time fixes and write long-term solutions instead.
- Separate files into directories in meaningful ways. Separate logic into helper functions in meaningful ways. Use concise but meaningful variable and function names.
