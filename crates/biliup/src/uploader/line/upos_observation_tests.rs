use super::{Bucket, Upos};
use crate::client::StatelessClient;
use crate::error::Kind;
use crate::uploader::line::UploadObserver;
use axum::http::StatusCode;
use bytes::Bytes;
use futures::TryStreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{Notify, mpsc};

#[derive(Default)]
struct Confirmations {
    bytes: AtomicU64,
    retries: AtomicUsize,
}
impl UploadObserver for Confirmations {
    fn chunk_confirmed(&self, _part_number: usize, bytes: usize) -> bool {
        self.bytes.fetch_add(bytes as u64, Ordering::SeqCst);
        true
    }
    fn chunk_retry(&self, _part_number: usize) {
        self.retries.fetch_add(1, Ordering::SeqCst);
    }
}

fn upos(addr: std::net::SocketAddr) -> Upos {
    Upos {
        client: StatelessClient::default(),
        bucket: Bucket {
            chunk_size: 4,
            auth: "test-auth".into(),
            endpoint: format!("//{addr}"),
            biz_id: 1,
            upos_uri: "upos://bucket/file.mp4".into(),
        },
        url: format!("http://{addr}/bucket/file.mp4"),
        upload_id: "test-upload".into(),
    }
}

#[tokio::test]
async fn bytes_are_observed_only_after_the_put_response_succeeds() {
    let (received, mut requests) = mpsc::unbounded_channel();
    let release = Arc::new(Notify::new());
    let server_release = release.clone();
    let app = axum::Router::new().route(
        "/bucket/file.mp4",
        axum::routing::put(move |body: Bytes| {
            let received = received.clone();
            let release = server_release.clone();
            async move {
                received.send(body.len()).unwrap();
                release.notified().await;
                StatusCode::OK
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let observations = Arc::new(Confirmations::default());
    let observed = observations.clone();
    let upload = tokio::spawn(async move {
        let upos = upos(addr);
        upos.upload_stream_with_observer(
            futures::stream::iter([Ok::<_, Kind>((Bytes::from_static(b"abcd"), 4))]),
            4,
            1,
            Some(observed.as_ref()),
        )
        .await?
        .try_collect::<Vec<_>>()
        .await
    });

    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), requests.recv())
            .await
            .unwrap(),
        Some(4),
    );
    // The body has been read and sent in full, but the server has not acknowledged it.
    assert_eq!(observations.bytes.load(Ordering::SeqCst), 0);
    release.notify_one();
    let parts = tokio::time::timeout(Duration::from_secs(5), upload)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(observations.bytes.load(Ordering::SeqCst), 4);
    assert_eq!(observations.retries.load(Ordering::SeqCst), 0);
    server.abort();
}

#[tokio::test]
async fn a_failed_put_retries_and_confirms_its_bytes_exactly_once() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let server_attempts = attempts.clone();
    let app = axum::Router::new().route(
        "/bucket/file.mp4",
        axum::routing::put(move |body: Bytes| {
            let attempts = server_attempts.clone();
            async move {
                assert_eq!(body.as_ref(), b"abcd");
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else {
                    StatusCode::OK
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let observations = Confirmations::default();
    let upos = upos(addr);
    let uploaded = tokio::time::timeout(Duration::from_secs(10), async {
        upos.upload_stream_with_observer(
            futures::stream::iter([Ok::<_, Kind>((Bytes::from_static(b"abcd"), 4))]),
            4,
            1,
            Some(&observations),
        )
        .await?
        .try_collect::<Vec<_>>()
        .await
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(uploaded.len(), 1);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(observations.retries.load(Ordering::SeqCst), 1);
    assert_eq!(observations.bytes.load(Ordering::SeqCst), 4);
    server.abort();
}
