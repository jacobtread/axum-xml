use std::net::Ipv4Addr;

use axum::{routing::post, Router};
use axum_xml_up::Xml;
use reqwest::{header, RequestBuilder, StatusCode};
use serde::Deserialize;
use tokio::{net::TcpListener, task::AbortHandle};

/// Testing harness for starting a server and
/// sending messages to the server
struct TestHarness {
    /// Base URL for accessing the server
    base_url: String,
    /// Handle to stop the server task
    abort_handle: AbortHandle,
    /// HTTP client for requesting the server
    client: reqwest::Client,
}

impl TestHarness {
    /// Creates a new test harness for the provided `router`
    async fn new(router: Router<()>) -> Self {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("Failed to start server socket");

        let addr = listener
            .local_addr()
            .expect("Failed to determine socket local address");

        println!("Test harness running on: {addr}");

        let abort_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("Error while running server");
        })
        .abort_handle();

        let client = reqwest::Client::new();

        let base_url = format!("http://{}", addr);

        Self {
            base_url,
            abort_handle,
            client,
        }
    }

    fn post(&self, path: &str) -> RequestBuilder {
        let base_url = &self.base_url;
        self.client.post(format!("{base_url}{path}"))
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        self.abort_handle.abort();
    }
}

/// Shared testing input structure for response handlers
#[derive(Debug, Deserialize)]
struct Input {
    #[serde(rename = "@foo")]
    foo: String,
}

/// Checks that a simple echo of the value of `foo` responds correctly
#[tokio::test]
async fn deserialize_body() {
    let router = Router::new().route("/", post(|Xml(input): Xml<Input>| async { input.foo }));
    let harness = TestHarness::new(router).await;
    let response = harness
        .post("/")
        .header(header::CONTENT_TYPE, "application/xml")
        .body(r#"<Input foo="bar"/>"#)
        .send()
        .await
        .expect("Failed to send request");

    let status = response.status();
    assert_eq!(status, StatusCode::OK);

    // Ensure the correct field
    let body = response.text().await.expect("Failed to get response text");
    assert_eq!(body, "bar");
}

/// Response should be an error if the XML content type was missing from
/// the request content type header
#[tokio::test]
async fn require_content_type() {
    let router = Router::new().route("/", post(|Xml(_): Xml<Input>| async {}));
    let harness = TestHarness::new(router).await;
    let response = harness
        .post("/")
        .body(r#"<Input foo="bar"/>"#)
        .send()
        .await
        .expect("Failed to send request");

    // Ensure the correct response status
    let status = response.status();
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);

    // Ensure the correct error response
    let body = response.text().await.expect("Failed to get response text");
    assert_eq!(
        body,
        "Expected request with `Content-Type: application/xml`"
    )
}

/// Tests a collection of valid and invalid content types to ensure the server
/// accepts all the valid types and rejects the invalid types
#[tokio::test]
async fn valid_content_types() {
    let router = Router::new().route("/", post(|Xml(input): Xml<Input>| async { input.foo }));
    let harness = TestHarness::new(router).await;

    async fn test_valid_content_type(harness: &TestHarness, content_type: &str, valid: bool) {
        let response = harness
            .post("/")
            .header(header::CONTENT_TYPE, content_type)
            .body(r#"<Input foo="bar"/>"#)
            .send()
            .await
            .expect("Failed to send request");

        let status = response.status();

        // React to response based on validity
        if valid {
            // Ensure the correct response status
            assert_eq!(status, StatusCode::OK);

            // Ensure the correct field
            let body = response.text().await.expect("Failed to get response text");
            assert_eq!(body, "bar");
        } else {
            // Ensure the correct response status
            assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);

            // Ensure the correct error response
            let body = response.text().await.expect("Failed to get response text");
            assert_eq!(
                body,
                "Expected request with `Content-Type: application/xml`"
            )
        }
    }

    let data: [(&str, bool); 6] = [
        ("application/xml", true),
        ("application/xml;charset=utf-8", true),
        ("application/cloudevents+xml", true),
        ("text/xml", true),
        ("application/json", false),
        ("text/html", false),
    ];

    for (content_type, valid) in data {
        test_valid_content_type(&harness, content_type, valid).await;
    }
}
