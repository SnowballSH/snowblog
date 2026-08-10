use jiff::Timestamp;
use metrics::set_default_local_recorder;
use metrics_exporter_prometheus::PrometheusBuilder;
use snowblog_core::domain::{Diagnostic, Language, PostStatus, Revision, Slug};
use snowblog_core::store::{
    AssetInput, NewPost, PostFilter, PostPatch, RenderArtifact, Store, StoreError, TranslationInput,
};

fn slug(s: &str) -> Slug {
    Slug::parse(s).unwrap()
}

fn lang(s: &str) -> Language {
    Language::parse(s).unwrap()
}

fn new_post(name: &str) -> NewPost {
    NewPost {
        slug: slug(name),
        default_language: lang("en"),
        tags: vec!["typst".into(), "blog".into()],
        published_at: None,
    }
}

fn translation(language: &str, source: &str) -> TranslationInput {
    TranslationInput {
        language: lang(language),
        title: format!("Title {language}"),
        description: format!("Description {language}"),
        source: source.to_string(),
    }
}

fn asset(path: &str, content: &[u8]) -> AssetInput {
    AssetInput {
        path: path.to_string(),
        content: content.to_vec(),
        content_type: "image/png".to_string(),
    }
}

fn artifact(html: &str) -> RenderArtifact {
    RenderArtifact {
        html: html.to_string(),
        renderer_version: "0.15.1".to_string(),
        input_hash: "deadbeef".to_string(),
        warnings: vec![Diagnostic::warning("test warning")],
        rendered_at: Timestamp::now(),
    }
}

#[tokio::test]
async fn create_then_get_round_trips() {
    let store = Store::in_memory().await.unwrap();
    let created = store.create_post(new_post("first_post")).await.unwrap();
    assert_eq!(created.post.slug.as_str(), "first_post");
    assert_eq!(created.post.status, PostStatus::Draft);
    assert_eq!(created.post.revision, Revision::INITIAL);
    assert_eq!(created.post.default_language, lang("en"));
    assert_eq!(created.post.tags, vec!["blog", "typst"]);
    assert!(created.post.published_at.is_none());
    assert!(created.translations.is_empty());
    assert!(created.renders.is_empty());
    assert!(created.asset_manifest.is_empty());

    let fetched = store.get_post(&slug("first_post")).await.unwrap().unwrap();
    assert_eq!(fetched.post.id, created.post.id);
    assert_eq!(fetched.post.created_at, created.post.created_at);
}

#[tokio::test(flavor = "current_thread")]
async fn every_public_store_boundary_records_exactly_once() {
    let store = Store::in_memory().await.unwrap();
    let recorder = PrometheusBuilder::new()
        .set_buckets(&[1.0])
        .expect("test buckets are non-empty")
        .build_recorder();
    let handle = recorder.handle();
    let _recorder_guard = set_default_local_recorder(&recorder);
    let post_slug = slug("observed_boundaries");

    let created = store
        .create_post(new_post(post_slug.as_str()))
        .await
        .unwrap();
    store.get_post(&post_slug).await.unwrap().unwrap();
    store.list_posts(PostFilter::default()).await.unwrap();
    store
        .upsert_translation(&post_slug, Revision(1), translation("zh", "= Observed"))
        .await
        .unwrap();
    store
        .update_post_meta(&post_slug, Revision(2), PostPatch::default())
        .await
        .unwrap();
    store
        .set_status(&post_slug, Revision(3), PostStatus::Draft, None)
        .await
        .unwrap();
    store
        .upsert_asset(
            &post_slug,
            Revision(4),
            asset("private/observed.png", b"observed"),
        )
        .await
        .unwrap();
    store
        .get_asset(&post_slug, "private/observed.png")
        .await
        .unwrap()
        .unwrap();
    store.get_assets(&post_slug).await.unwrap();
    assert!(
        store
            .replace_render(
                &created.post.id,
                &lang("zh"),
                Revision(5),
                artifact("<p>observed</p>"),
            )
            .await
            .unwrap()
    );
    store
        .delete_asset(&post_slug, Revision(5), "private/observed.png")
        .await
        .unwrap();
    store
        .delete_translation(&post_slug, Revision(6), &lang("zh"))
        .await
        .unwrap();
    store.delete_post(&post_slug, Revision(7)).await.unwrap();

    let exposition = handle.render();
    for operation in [
        "get_post",
        "list_posts",
        "create_post",
        "update_post_meta",
        "set_status",
        "delete_post",
        "save_translation",
        "delete_translation",
        "save_asset",
        "delete_asset",
        "get_asset",
        "get_assets",
        "replace_render",
    ] {
        assert_eq!(
            metric_sample(
                &exposition,
                "snowblog_store_operations_total",
                &[("operation", operation), ("result", "ok")],
            ),
            1.0,
            "unexpected operation count for {operation}"
        );
        assert_eq!(
            metric_sample(
                &exposition,
                "snowblog_store_operation_duration_seconds_count",
                &[("operation", operation)],
            ),
            1.0,
            "unexpected duration count for {operation}"
        );
    }
    assert!(
        !exposition
            .lines()
            .any(|line| line.starts_with("snowblog_store_operations_total")
                && line.contains("result=\"error\"")),
        "successful store calls emitted an error result"
    );
    for forbidden in ["observed_boundaries", "private/observed.png", "Observed"] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }
}

#[tokio::test]
async fn duplicate_slug_rejected() {
    let store = Store::in_memory().await.unwrap();
    store.create_post(new_post("taken")).await.unwrap();
    let error = store.create_post(new_post("taken")).await.unwrap_err();
    assert!(matches!(error, StoreError::SlugTaken(s) if s.as_str() == "taken"));
}

#[tokio::test]
async fn wrong_revision_rejected_and_leaves_post_unchanged() {
    let store = Store::in_memory().await.unwrap();
    store.create_post(new_post("guarded")).await.unwrap();
    let s = slug("guarded");
    let wrong = Revision(99);

    let meta = store
        .update_post_meta(
            &s,
            wrong,
            PostPatch {
                tags: Some(vec!["x".into()]),
                ..Default::default()
            },
        )
        .await;
    assert!(
        matches!(
            meta,
            Err(StoreError::RevisionMismatch {
                actual: Revision(1),
                ..
            })
        ),
        "update_post_meta accepted a stale revision: {meta:?}"
    );
    let tr = store
        .upsert_translation(&s, wrong, translation("en", "= Hi"))
        .await;
    assert!(
        matches!(tr, Err(StoreError::RevisionMismatch { .. })),
        "upsert_translation accepted a stale revision"
    );
    let st = store
        .set_status(&s, wrong, PostStatus::Published, None)
        .await;
    assert!(
        matches!(st, Err(StoreError::RevisionMismatch { .. })),
        "set_status accepted a stale revision"
    );
    let up = store
        .upsert_asset(&s, wrong, asset("assets/a.png", b"x"))
        .await;
    assert!(
        matches!(up, Err(StoreError::RevisionMismatch { .. })),
        "upsert_asset accepted a stale revision"
    );
    let del = store.delete_post(&s, wrong).await;
    assert!(
        matches!(del, Err(StoreError::RevisionMismatch { .. })),
        "delete_post accepted a stale revision"
    );

    let record = store.get_post(&s).await.unwrap().unwrap();
    assert_eq!(record.post.revision, Revision(1));
    assert_eq!(record.post.tags, vec!["blog", "typst"]);
    assert!(record.translations.is_empty());
}

#[tokio::test]
async fn mutations_bump_revision_by_one() {
    let store = Store::in_memory().await.unwrap();
    store.create_post(new_post("bumpy")).await.unwrap();
    let s = slug("bumpy");

    let r2 = store
        .upsert_translation(&s, Revision(1), translation("en", "= A"))
        .await
        .unwrap();
    assert_eq!(r2.post.revision, Revision(2));
    let r3 = store
        .upsert_asset(&s, Revision(2), asset("assets/a.png", b"abc"))
        .await
        .unwrap();
    assert_eq!(r3.post.revision, Revision(3));
    let r4 = store
        .update_post_meta(
            &s,
            Revision(3),
            PostPatch {
                tags: Some(vec!["t".into()]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(r4.post.revision, Revision(4));
    let r5 = store
        .delete_asset(&s, Revision(4), "assets/a.png")
        .await
        .unwrap();
    assert_eq!(r5.post.revision, Revision(5));
}

#[tokio::test]
async fn delete_default_translation_refused() {
    let store = Store::in_memory().await.unwrap();
    store.create_post(new_post("langs")).await.unwrap();
    let s = slug("langs");
    store
        .upsert_translation(&s, Revision(1), translation("en", "= A"))
        .await
        .unwrap();
    store
        .upsert_translation(&s, Revision(2), translation("zh", "= B"))
        .await
        .unwrap();

    let error = store
        .delete_translation(&s, Revision(3), &lang("en"))
        .await
        .unwrap_err();
    assert!(matches!(error, StoreError::DefaultLanguageViolation));
    let ok = store
        .delete_translation(&s, Revision(3), &lang("zh"))
        .await
        .unwrap();
    assert_eq!(ok.translations.len(), 1);
}

#[tokio::test]
async fn default_language_change_requires_translation() {
    let store = Store::in_memory().await.unwrap();
    store.create_post(new_post("switching")).await.unwrap();
    let s = slug("switching");
    store
        .upsert_translation(&s, Revision(1), translation("en", "= A"))
        .await
        .unwrap();

    let patch = PostPatch {
        default_language: Some(lang("zh")),
        ..Default::default()
    };
    let error = store
        .update_post_meta(&s, Revision(2), patch.clone())
        .await
        .unwrap_err();
    assert!(matches!(error, StoreError::DefaultLanguageViolation));

    store
        .upsert_translation(&s, Revision(2), translation("zh", "= B"))
        .await
        .unwrap();
    let updated = store
        .update_post_meta(&s, Revision(3), patch)
        .await
        .unwrap();
    assert_eq!(updated.post.default_language, lang("zh"));
}

#[tokio::test]
async fn list_filters_and_orders() {
    let store = Store::in_memory().await.unwrap();
    for (name, days_ago) in [("post_a", 3), ("post_b", 1), ("post_c", 2)] {
        let mut post = new_post(name);
        post.published_at =
            Some(Timestamp::now() - jiff::SignedDuration::from_hours(24 * days_ago));
        if name == "post_c" {
            post.tags = vec!["special".into()];
        }
        store.create_post(post).await.unwrap();
        store
            .set_status(&slug(name), Revision(1), PostStatus::Published, None)
            .await
            .unwrap();
    }
    store.create_post(new_post("draft_post")).await.unwrap();

    let published = store
        .list_posts(PostFilter {
            status: Some(PostStatus::Published),
            ..Default::default()
        })
        .await
        .unwrap();
    let slugs: Vec<&str> = published.iter().map(|r| r.post.slug.as_str()).collect();
    assert_eq!(slugs, vec!["post_b", "post_c", "post_a"]);

    let tagged = store
        .list_posts(PostFilter {
            tag: Some("special".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].post.slug.as_str(), "post_c");

    let limited = store
        .list_posts(PostFilter {
            status: Some(PostStatus::Published),
            limit: 1,
            offset: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].post.slug.as_str(), "post_c");
}

#[tokio::test]
async fn delete_post_cascades() {
    let store = Store::in_memory().await.unwrap();
    let created = store.create_post(new_post("doomed")).await.unwrap();
    let s = slug("doomed");
    store
        .upsert_translation(&s, Revision(1), translation("en", "= A"))
        .await
        .unwrap();
    store
        .upsert_asset(&s, Revision(2), asset("assets/a.png", b"abc"))
        .await
        .unwrap();
    store
        .replace_render(
            &created.post.id,
            &lang("en"),
            Revision(3),
            artifact("<p>hi</p>"),
        )
        .await
        .unwrap();

    store.delete_post(&s, Revision(3)).await.unwrap();
    assert!(store.get_post(&s).await.unwrap().is_none());
    assert!(store.get_asset(&s, "assets/a.png").await.unwrap().is_none());
    for (table, count_sql) in [
        ("post_tags", "SELECT COUNT(*) FROM post_tags"),
        (
            "post_translations",
            "SELECT COUNT(*) FROM post_translations",
        ),
        ("renders", "SELECT COUNT(*) FROM renders"),
        ("assets", "SELECT COUNT(*) FROM assets"),
    ] {
        let count: i64 = sqlx::query_scalar(count_sql)
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(count, 0, "{table} not cascaded");
    }
}

#[tokio::test]
async fn foreign_keys_enforced() {
    let store = Store::in_memory().await.unwrap();
    let result = sqlx::query("INSERT INTO post_tags (post_id, tag) VALUES ('nope', 'x')")
        .execute(store.pool())
        .await;
    assert!(
        result.is_err(),
        "orphan post_tags row accepted: foreign keys are off"
    );
}

#[tokio::test]
async fn slug_rename_moves_post() {
    let store = Store::in_memory().await.unwrap();
    store.create_post(new_post("old_name")).await.unwrap();
    store
        .update_post_meta(
            &slug("old_name"),
            Revision(1),
            PostPatch {
                slug: Some(slug("new_name")),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(store.get_post(&slug("old_name")).await.unwrap().is_none());
    let renamed = store.get_post(&slug("new_name")).await.unwrap().unwrap();
    assert_eq!(renamed.post.revision, Revision(2));

    store.create_post(new_post("old_name")).await.unwrap();
    let clash = store
        .update_post_meta(
            &slug("new_name"),
            Revision(2),
            PostPatch {
                slug: Some(slug("old_name")),
                ..Default::default()
            },
        )
        .await;
    assert!(matches!(clash, Err(StoreError::SlugTaken(_))));
}

#[tokio::test]
async fn render_round_trips() {
    let store = Store::in_memory().await.unwrap();
    let created = store.create_post(new_post("rendered")).await.unwrap();
    let s = slug("rendered");
    store
        .upsert_translation(&s, Revision(1), translation("en", "= A"))
        .await
        .unwrap();
    let stored = store
        .replace_render(
            &created.post.id,
            &lang("en"),
            Revision(2),
            artifact("<p>body</p>"),
        )
        .await
        .unwrap();
    assert!(stored);

    let stale_write = store
        .replace_render(
            &created.post.id,
            &lang("en"),
            Revision(1),
            artifact("<p>stale</p>"),
        )
        .await
        .unwrap();
    assert!(
        !stale_write,
        "a stale snapshot must not overwrite the artifact"
    );

    let record = store.get_post(&s).await.unwrap().unwrap();
    assert_eq!(
        record.post.revision,
        Revision(2),
        "replace_render must not bump revision"
    );
    let render = record.render(&lang("en")).unwrap();
    assert_eq!(render.html, "<p>body</p>");
    assert_eq!(render.renderer_version, "0.15.1");
    assert_eq!(render.input_hash, "deadbeef");
    assert_eq!(render.warnings.len(), 1);
}

#[tokio::test]
async fn publish_sets_published_at_only_if_unset() {
    let store = Store::in_memory().await.unwrap();
    store.create_post(new_post("dated")).await.unwrap();
    let s = slug("dated");
    let first = Timestamp::now();
    let published = store
        .set_status(&s, Revision(1), PostStatus::Published, Some(first))
        .await
        .unwrap();
    assert_eq!(published.post.published_at, Some(first));

    let later = Timestamp::now() + jiff::SignedDuration::from_hours(1);
    let republished = store
        .set_status(&s, Revision(2), PostStatus::Published, Some(later))
        .await
        .unwrap();
    assert_eq!(republished.post.published_at, Some(first));
}

fn metric_sample(exposition: &str, family: &str, labels: &[(&str, &str)]) -> f64 {
    let line = exposition
        .lines()
        .filter(|line| line.starts_with(family))
        .find(|line| {
            labels
                .iter()
                .all(|(name, value)| line.contains(&format!("{name}=\"{value}\"")))
        })
        .unwrap_or_else(|| panic!("missing {family} sample for {labels:?}"));
    line.rsplit_once(' ')
        .expect("Prometheus sample has a value")
        .1
        .parse()
        .expect("Prometheus sample value is numeric")
}
