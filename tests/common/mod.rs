use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// Test helper: send a GET request to the app and return (status, body).
pub async fn get(app: &Router, uri: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap_or_default();

    let response = app.clone().oneshot(request).await.unwrap_or_default();

    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .map(http_body_util::Collected::to_bytes)
        .unwrap_or_default();
    let body_str = String::from_utf8(body.to_vec()).unwrap_or_default();

    (status, body_str)
}
