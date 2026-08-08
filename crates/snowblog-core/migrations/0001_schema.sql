CREATE TABLE posts (
    id               TEXT PRIMARY KEY,
    slug             TEXT NOT NULL UNIQUE,
    status           TEXT NOT NULL CHECK (status IN ('draft','published','archived')),
    default_language TEXT NOT NULL,
    revision         INTEGER NOT NULL DEFAULT 1,
    published_at     TEXT,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);

CREATE TABLE post_tags (
    post_id TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tag     TEXT NOT NULL,
    PRIMARY KEY (post_id, tag)
);
CREATE INDEX idx_post_tags_tag ON post_tags(tag);

CREATE TABLE post_translations (
    post_id     TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    language    TEXT NOT NULL,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    source      TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    PRIMARY KEY (post_id, language)
);

CREATE TABLE renders (
    post_id          TEXT NOT NULL,
    language         TEXT NOT NULL,
    html             TEXT NOT NULL,
    renderer_version TEXT NOT NULL,
    input_hash       TEXT NOT NULL,
    warnings         TEXT NOT NULL DEFAULT '[]',
    rendered_at      TEXT NOT NULL,
    PRIMARY KEY (post_id, language),
    FOREIGN KEY (post_id, language)
        REFERENCES post_translations(post_id, language) ON DELETE CASCADE
);

CREATE TABLE assets (
    post_id      TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    path         TEXT NOT NULL,
    content      BLOB NOT NULL,
    content_type TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    PRIMARY KEY (post_id, path)
);
