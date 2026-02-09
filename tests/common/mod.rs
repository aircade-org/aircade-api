use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[allow(dead_code)]
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

#[allow(dead_code)]
/// Test helper: send a POST request with JSON body and return (status, body).
pub async fn post_json(app: &Router, uri: &str, body: &serde_json::Value) -> (StatusCode, String) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap_or_default()))
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

#[allow(dead_code)]
/// Test helper: send a POST request with JSON body and auth token.
pub async fn post_json_with_auth(
    app: &Router,
    uri: &str,
    body: &serde_json::Value,
    token: &str,
) -> (StatusCode, String) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(serde_json::to_string(body).unwrap_or_default()))
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

#[allow(dead_code)]
/// Test helper: send a DELETE request with auth token.
pub async fn delete_with_auth(app: &Router, uri: &str, token: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .method("DELETE")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
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

#[allow(dead_code)]
/// Test helper: send a GET request with auth token.
pub async fn get_with_auth(app: &Router, uri: &str, token: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
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
