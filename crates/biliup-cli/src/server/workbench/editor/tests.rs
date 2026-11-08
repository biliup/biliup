use super::*;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::permissions::{Permission, required_permission};
use crate::server::workbench::{retention, store};
use serde_json::json;

#[tokio::test]
async fn imported_media_are_validated_without_a_recording_session() {
    let (dir, pool, mut doc, _) = fixture().await;
    let path = dir.path().join("image.png");
    std::fs::write(&path, b"fixture").unwrap();
    sqlx::query("INSERT INTO editor_assets(id,path,file_name,kind,duration_ms,probe_json,created_at) VALUES('photo',?,'photo.png','image',NULL,'{}',1)")
        .bind(path.to_string_lossy().as_ref()).execute(&pool).await.unwrap();
    doc.origin = Origin {
        session_id: None,
        in_ms: 0,
        out_ms: 0,
    };
    doc.sources = BTreeMap::from([(
        "file-photo".into(),
        MediaSource::File(FileSource {
            kind: "file".into(),
            asset_id: "photo".into(),
            duration_ms: None,
            file_name: "ignored".into(),
        }),
    )]);
    doc.project["media"] =
        json!([{"id":"file-photo","kind":"image","name":"Photo","path":"imports/file-photo"}]);
    doc.project["tracks"][0]["items"] =
        json!([{"id":"photo-clip","type":"clip","media":"file-photo","duration":900}]);
    let saved = save(&pool, None, doc.clone()).await.unwrap();
    assert_eq!(
        media::source(&pool, saved.summary.id, "file-photo")
            .await
            .unwrap()
            .0,
        path
    );
    let MediaSource::File(source) = &saved.document.sources["file-photo"] else {
        panic!()
    };
    assert_eq!(source.file_name, "photo.png");
    doc.project["media"][0]["path"] = json!("../../private");
    assert!(matches!(
        save(&pool, None, doc).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        required_permission(
            &axum::http::Method::POST,
            "/v1/editor/projects/{id}/imports",
            "/v1/editor/projects/1/imports"
        ),
        Some(Permission::ClipEdit)
    );
}

pub(super) async fn fixture() -> (
    tempfile::TempDir,
    ConnectionPool,
    Document,
    std::path::PathBuf,
) {
    let dir = tempfile::tempdir().unwrap();
    let pool = ConnectionManager::new_pool(dir.path().join("db.sqlite").to_str().unwrap())
        .await
        .unwrap();
    let session: i64 = sqlx::query_scalar("INSERT INTO stream_sessions(name,url,title,date,live_cover_path,started_at,ended_at) VALUES('test','https://test','test','2026-10-24','',1000,11000) RETURNING id").fetch_one(&pool).await.unwrap();
    let path = dir.path().join("source.mp4");
    let output = crate::tools::ffmpeg_command()
        .args([
            "-v",
            "error",
            "-nostdin",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=64x64:r=30:d=10",
            "-c:v",
            "mpeg4",
            "-threads",
            "1",
        ])
        .arg(&path)
        .output()
        .await
        .expect("FFmpeg is required to create the editor recording fixture");
    assert!(
        output.status.success(),
        "failed to create editor recording: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let segment: i64 = sqlx::query_scalar("INSERT INTO segments(session_id,path,container,state,start_ms,end_ms) VALUES(?,?,'mp4','finished',0,10000) RETURNING id")
        .bind(session).bind(path.to_string_lossy().as_ref()).fetch_one(&pool).await.unwrap();
    let id = format!("recording-{session}-{segment}");
    let doc: Document = serde_json::from_value(json!({
        "format":"biliup.editor/1", "origin":{"sessionId":session,"inMs":0,"outMs":10000},
        "sourceMarkers":[],"warnings":[],
        "project":{"format":"biliup.timeline/1","name":"test", "canvas":{"width":1920,"height":1080,"fps":30},
            "media":[{"id":id,"name":"source.mp4","kind":"video","path":format!("recordings/{id}.mp4")}],
            "tracks":[{"id":"main","name":"主视频","kind":"video","role":"main","items":[{"id":"clip","type":"clip","media":id,"in":0,"duration":300}]}]},
        "sources":{id:{"kind":"recording","sessionId":session,"segmentId":segment,"viewSessionId":session,
            "viewStartMs":0,"sourceStartMs":0,"durationMs":10000,"fileName":"forged-label","state":"finished","hasDanmaku":true}}
    })).unwrap();
    (dir, pool, doc, path)
}

#[tokio::test]
async fn saves_history_and_rejects_stale_writes_and_deletes() {
    let (_dir, pool, doc, _) = fixture().await;
    let first = save(&pool, None, doc).await.unwrap();
    assert_eq!(first.summary.revision, 1);
    assert_eq!(
        first.document.project["media"][0]["probe"],
        json!({"width":64,"height":64,"fps":30.0,"hasAudio":false,"frames":300})
    );
    assert_eq!(
        first
            .document
            .sources
            .values()
            .next()
            .unwrap()
            .recording()
            .unwrap()
            .file_name,
        "source.mp4"
    );
    assert!(
        !first
            .document
            .sources
            .values()
            .next()
            .unwrap()
            .recording()
            .unwrap()
            .has_danmaku
    );
    let mut renamed = first.document.clone();
    renamed.project["name"] = json!("renamed");
    let second = save(&pool, Some((first.summary.id, 1)), renamed.clone())
        .await
        .unwrap();
    assert_eq!(second.summary.revision, 2);
    assert!(matches!(
        save(&pool, Some((first.summary.id, 1)), renamed).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        delete(&pool, first.summary.id, 1).await,
        Err(Error::Conflict)
    ));
    assert_eq!(
        revision(&pool, first.summary.id, 1).await.unwrap().project["name"],
        "test"
    );
    assert_eq!(
        get(&pool, first.summary.id).await.unwrap().document.project["name"],
        "renamed"
    );
    assert_eq!(list(&pool, None, None).await.unwrap().len(), 1);
}

#[tokio::test]
async fn concurrent_saves_have_one_winner() {
    let (_dir, pool, doc, _) = fixture().await;
    let first = save(&pool, None, doc).await.unwrap();
    let (a, b) = tokio::join!(
        save(&pool, Some((first.summary.id, 1)), first.document.clone()),
        save(&pool, Some((first.summary.id, 1)), first.document)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(a, Err(Error::Conflict)) || matches!(b, Err(Error::Conflict)));
}

#[tokio::test]
async fn project_listing_filters_before_cursor_and_delete_preserves_outputs() {
    let (_dir, pool, mut doc, recording) = fixture().await;
    let mut ids = Vec::new();
    for i in 0..23 {
        doc.project["name"] = json!(format!("Search%_Project {i}"));
        ids.push(save(&pool, None, doc.clone()).await.unwrap().summary.id);
    }
    doc.project["name"] = json!("unrelated");
    save(&pool, None, doc.clone()).await.unwrap();
    let recent = list_filtered(&pool, None, None, "", 6).await.unwrap();
    assert_eq!(recent.len(), 6);
    let page = list_filtered(&pool, None, None, "search%_project", 21)
        .await
        .unwrap();
    assert_eq!(page.len(), 21);
    assert_eq!(page[0].id, ids[22]);
    let next = list_filtered(&pool, None, Some(page[19].id), "search%_project", 21)
        .await
        .unwrap();
    assert_eq!(
        next.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![ids[2], ids[1], ids[0]]
    );
    assert!(
        list_filtered(&pool, None, None, "no-such-project", 6)
            .await
            .unwrap()
            .is_empty()
    );
    let id = ids[0];
    sqlx::query("INSERT INTO editor_exports(project_id,revision,phase,job_json,document_json,options_json,created_at) VALUES(?,1,'queued','{}','{}','{}',1)").bind(id).execute(&pool).await.unwrap();
    assert!(matches!(delete(&pool, id, 1).await, Err(Error::Invalid(_))));
    sqlx::query("UPDATE editor_exports SET phase='done' WHERE project_id=?")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    delete(&pool, id, 1).await.unwrap();
    assert!(matches!(get(&pool, id).await, Err(Error::NotFound)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM editor_revisions WHERE project_id=?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM editor_exports WHERE project_id=?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(recording.exists());
}

#[tokio::test]
async fn bad_schema_paths_ownership_and_prototype_keys_are_atomic() {
    let (_dir, pool, doc, _) = fixture().await;
    for (pointer, value) in [
        ("/media/0/path", json!("../../private.mp4")),
        ("/canvas/fps", json!(0)),
        ("/tracks/0/items/0/media", json!("unknown")),
        ("/tracks/0/id", json!("clip")),
    ] {
        let mut bad = doc.clone();
        *bad.project.pointer_mut(pointer).unwrap() = value;
        assert!(matches!(
            save(&pool, None, bad).await,
            Err(Error::Invalid(_))
        ));
    }
    let mut bad = doc.clone();
    let MediaSource::Recording(source) = bad.sources.values_mut().next().unwrap() else {
        panic!("expected recording")
    };
    source.session_id += 1;
    assert!(matches!(
        save(&pool, None, bad).await,
        Err(Error::Invalid(_))
    ));
    let mut bad = doc;
    bad.project["theme"] = json!({"colors":{"__proto__":"bad"}});
    assert!(matches!(
        save(&pool, None, bad).await,
        Err(Error::Invalid(_))
    ));
    assert!(list(&pool, None, None).await.unwrap().is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM editor_sources")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn saved_history_protects_media_from_all_cleanup_until_project_deletion() {
    let (_dir, pool, doc, path) = fixture().await;
    let session = doc.origin.session_id.unwrap();
    let first = save(&pool, None, doc).await.unwrap();
    // The latest version no longer uses this recording; revision 1 still does.
    // Removing a processed output must not indirectly dispose of a project input.
    let output = path.with_file_name("derived.mp4");
    std::fs::write(&output, b"derived").unwrap();
    let mut conn = pool.acquire().await.unwrap();
    retention::record_processed_sources(&mut conn, &output, std::slice::from_ref(&path))
        .await
        .unwrap();
    drop(conn);
    retention::remove(
        &retention::Retention::without_delay(pool.clone()),
        &[output.as_path()],
    )
    .await
    .unwrap();
    assert!(path.exists());
    assert!(!output.exists());
    let mut removed = first.document.clone();
    removed.sources.clear();
    removed.project["media"] = json!([]);
    removed.project["tracks"][0]["items"] = json!([]);
    save(&pool, Some((first.summary.id, 1)), removed)
        .await
        .unwrap();
    assert_eq!(
        retention::remove(
            &retention::Retention::without_delay(pool.clone()),
            &[path.as_path()]
        )
        .await
        .unwrap(),
        vec![retention::Disposal::Deferred]
    );
    assert_eq!(retention::sweep_pending(&pool, i64::MAX).await.unwrap(), 0);
    assert_eq!(
        retention::enforce_free_space(&pool, 1024, i64::MAX, |_| Ok(0))
            .await
            .unwrap(),
        0
    );
    assert!(path.exists());
    assert_eq!(
        store::delete_session(&pool, session).await.unwrap(),
        store::DeleteSessionOutcome::EditorReferenced
    );
    delete(&pool, first.summary.id, 2).await.unwrap();
    assert!(matches!(
        revision(&pool, first.summary.id, 1).await,
        Err(Error::NotFound)
    ));
    assert_eq!(retention::sweep_pending(&pool, i64::MAX).await.unwrap(), 1);
    assert!(!path.exists());
    assert_eq!(
        store::delete_session(&pool, session).await.unwrap(),
        store::DeleteSessionOutcome::Deleted
    );
}

#[tokio::test]
async fn merged_source_coordinates_are_checked() {
    let (_dir, pool, mut doc, _) = fixture().await;
    let owner = doc.origin.session_id.unwrap();
    // Keep the source coordinates distinct from the merged view coordinates.
    sqlx::query("INSERT INTO livestreamers(id,url,remark) VALUES(1,'https://test','test')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE stream_sessions SET streamer_id=1 WHERE id=?")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    let group: i64 = sqlx::query_scalar("INSERT INTO stream_sessions(name,url,title,date,live_cover_path,started_at,ended_at,kind) VALUES('group','','group','2026-10-24','',0,16000,'group') RETURNING id").fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO session_group_members(group_session_id,source_session_id,ordinal,offset_ms) VALUES(?,?,0,5000)").bind(group).bind(owner).execute(&pool).await.unwrap();
    doc.origin.session_id = Some(group);
    doc.origin.in_ms = 5000;
    doc.origin.out_ms = 15000;
    let MediaSource::Recording(source) = doc.sources.values_mut().next().unwrap() else {
        panic!("expected recording")
    };
    source.view_session_id = group;
    source.view_start_ms = 5000;
    let saved = save(&pool, None, doc.clone()).await.unwrap();
    assert_eq!(saved.summary.session_id, Some(group));
    let MediaSource::Recording(source) = doc.sources.values_mut().next().unwrap() else {
        panic!("expected recording")
    };
    source.source_start_ms = 5000;
    assert!(matches!(
        save(&pool, None, doc).await,
        Err(Error::Invalid(_))
    ));
}

#[test]
fn routes_preserve_workbench_permissions() {
    use axum::http::Method;
    for route in [
        "/v1/editor/projects",
        "/v1/editor/projects/{id}",
        "/v1/editor/projects/{id}/revisions/{revision}",
        "/v1/editor/projects/{id}/exports",
        "/v1/outputs",
        "/v1/editor/projects/{id}/exports/{job}/download",
    ] {
        assert_eq!(
            required_permission(&Method::GET, route, route),
            Some(Permission::FileView)
        );
    }
    for (method, route) in [
        (Method::POST, "/v1/editor/projects"),
        (Method::POST, "/v1/editor/projects/{id}/exports"),
        (
            Method::POST,
            "/v1/editor/projects/{id}/exports/{job}/cancel",
        ),
        (Method::PUT, "/v1/editor/projects/{id}"),
        (Method::DELETE, "/v1/editor/projects/{id}"),
    ] {
        assert_eq!(
            required_permission(&method, route, route),
            Some(Permission::ClipEdit)
        );
    }
}

#[tokio::test]
async fn empty_project_can_add_recordings_and_keep_revision_retention() {
    let (_dir, pool, doc, _) = fixture().await;
    let mut empty = doc.clone();
    empty.origin = Origin {
        session_id: None,
        in_ms: 0,
        out_ms: 0,
    };
    empty.sources.clear();
    empty.project["media"] = json!([]);
    empty.project["tracks"][0]["items"] = json!([]);
    let saved = save(&pool, None, empty).await.unwrap();
    assert_eq!(saved.summary.session_id, None);
    let added = save(&pool, Some((saved.summary.id, 1)), doc.clone())
        .await
        .unwrap();
    assert_eq!(added.summary.session_id, doc.origin.session_id);
    assert_eq!(
        revision(&pool, saved.summary.id, 1)
            .await
            .unwrap()
            .origin
            .session_id,
        None
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM editor_sources WHERE project_id=?")
            .bind(saved.summary.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn source_view_uses_session_membership_without_creating_projects() {
    let (_dir, pool, doc, path) = fixture().await;
    let media = doc.sources.keys().next().unwrap();
    let session = doc.origin.session_id.unwrap();
    let (resolved, source) = media::session_source(&pool, session, media).await.unwrap();
    assert_eq!(resolved, path);
    assert_eq!(source.duration_ms, 10000);
    assert!(list(&pool, None, None).await.unwrap().is_empty());
    assert!(matches!(
        media::session_source(&pool, session + 100, media).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        media::session_source(&pool, session, &media.replace("recording", "wrong")).await,
        Err(Error::NotFound)
    ));
    let group: i64 = sqlx::query_scalar("INSERT INTO stream_sessions(name,url,title,date,live_cover_path,started_at,ended_at,kind) VALUES('group','','group','2026-10-24','',0,16000,'group') RETURNING id").fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO session_group_members(group_session_id,source_session_id,ordinal,offset_ms) VALUES(?,?,0,5000)").bind(group).bind(session).execute(&pool).await.unwrap();
    assert_eq!(
        media::session_source(&pool, group, media)
            .await
            .unwrap()
            .1
            .view_start_ms,
        5000
    );
    assert_eq!(
        required_permission(
            &axum::http::Method::GET,
            "/v1/editor/sessions/{id}/media/{media}/{kind}",
            "/v1/editor/sessions/1/media/recording-1-1/file"
        ),
        Some(Permission::FileView)
    );
}
