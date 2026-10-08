//! A bundled, read-only setup guide and SDK demo. No workspace files are served.
use axum::{extract::OriginalUri, http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}, routing::get, Router};
pub fn router() -> Router {
    Router::new().route("/setup", get(|| async { axum::response::Redirect::temporary("/setup/") }))
        .route("/setup/", get(asset)).route("/setup/{*path}", get(asset))
}
async fn asset(headers: HeaderMap, OriginalUri(uri): OriginalUri) -> Response {
    if headers.get_all("host").iter().count() != 1 || headers.get("host").and_then(|h| h.to_str().ok()) != Some("127.0.0.1:47832") {
        return StatusCode::FORBIDDEN.into_response();
    }
    let (kind, body) = match uri.path() {
        "/setup/" | "/setup/index.html" => ("text/html; charset=utf-8", include_str!("../../site/index.html")),
        "/setup/style.css" => ("text/css; charset=utf-8", include_str!("../../site/style.css")),
        "/setup/app.js" => ("text/javascript; charset=utf-8", include_str!("../../site/app.js")),
        "/setup/release.config.json" => ("application/json", include_str!("../../release.config.json")),
        "/setup/sdk/index.js" => ("text/javascript; charset=utf-8", include_str!("../../extensions/sdk/dist/index.js")),
        "/setup/sdk/client.js" => ("text/javascript; charset=utf-8", include_str!("../../extensions/sdk/dist/client.js")),
        "/setup/sdk/types.js" => ("text/javascript; charset=utf-8", include_str!("../../extensions/sdk/dist/types.js")),
        "/setup/sdk/icons.js" => ("text/javascript; charset=utf-8", include_str!("../../extensions/sdk/dist/icons.js")),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    ([ ("Content-Type", kind), ("Cache-Control", "no-store"), ("X-Content-Type-Options", "nosniff"),
       ("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: blob:; connect-src 'self' http://127.0.0.1:47832; frame-ancestors 'none'") ], body).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use tower::ServiceExt;
    #[tokio::test]
    async fn setup_serves_only_bundled_assets_and_checks_host() {
        for path in ["/setup/", "/setup/app.js", "/setup/sdk/client.js"] {
            let response = router().oneshot(axum::http::Request::builder().uri(path).header("host", "127.0.0.1:47832").body(Body::empty()).unwrap()).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert!(!to_bytes(response.into_body(), 100_000).await.unwrap().is_empty());
        }
        let response = router().oneshot(axum::http::Request::builder().uri("/setup/unknown.db").header("host", "127.0.0.1:47832").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = router().oneshot(axum::http::Request::builder().uri("/setup/").header("host", "untrusted.example").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
