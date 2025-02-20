use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Handler for the root path (/), returns a simple HTML page
///
/// # Examples
///
/// ```no_run
/// use axum::{
///     Router,
///     routing::get,
/// };
///
/// async fn test_index() {
///     let app = Router::new().route("/", get(lion_ui::index_handler));
///     // In a real server, we would bind and serve
/// }
/// ```
pub async fn index_handler() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <title>lion UI</title>
</head>
<body>
    <h1>lion UI - Hello from Phase1</h1>
    <button id="pingBtn">Ping Microkernel</button>
    <div id="resp"></div>

    <script>
        async function ping() {
            const respEle = document.getElementById('resp');
            try {
                const res = await fetch('/ping');
                const text = await res.text();
                respEle.textContent = text;
            } catch (e) {
                respEle.textContent = 'Error: ' + e;
            }
        }

        document.getElementById('pingBtn').onclick = ping;
    </script>
</body>
</html>"#;
    Html(html)
}

/// Handler for the /ping endpoint, returns a simple response from the microkernel
///
/// # Examples
///
/// ```no_run
/// use axum::{
///     Router,
///     routing::get,
/// };
///
/// async fn test_ping() {
///     let app = Router::new().route("/ping", get(lion_ui::ping_handler));
///     // In a real server, we would bind and serve
/// }
/// ```
pub async fn ping_handler() -> &'static str {
    // TODO: In future phases, this will call actual microkernel functions
    "Pong from lion_ui microkernel!"
}

#[tokio::main]
async fn main() {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .compact()
        .init();

    info!("Starting lion_ui server...");

    // Build our application router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ping", get(ping_handler));

    // Run it on localhost:8080
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Server started successfully");

    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_ping_endpoint() {
        let app = Router::new().route("/ping", get(ping_handler));

        let response = app
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();

        assert_eq!(body, "Pong from lion_ui microkernel!");
    }

    #[tokio::test]
    async fn test_index_endpoint() {
        let app = Router::new().route("/", get(index_handler));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();

        // Check that the response contains our expected HTML elements
        assert!(body.contains("lion UI - Hello from Phase1"));
        assert!(body.contains("Ping Microkernel"));
    }
}
