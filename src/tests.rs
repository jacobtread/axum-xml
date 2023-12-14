use crate::Xml;
use axum::routing::post;
use axum::Router;
use reqwest::{RequestBuilder, StatusCode};
use serde::Deserialize;
use std::net::SocketAddr;
use std::{assert_eq, println};
use tokio::net::TcpListener;

pub struct TestClient {
    client: reqwest::Client,
    addr: SocketAddr,
}

impl TestClient {
    #[allow(clippy::type_repetition_in_bounds)]
    pub(crate) async fn new(svc: Router<()>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Could not bind ephemeral socket");
        let addr = listener.local_addr().unwrap();
        println!("Listening on {}", addr);

        tokio::spawn(async move {
            axum::serve(listener, svc.into_make_service())
                .await
                .expect("server error");
        });

        Self {
            client: reqwest::Client::new(),
            addr,
        }
    }

    pub(crate) fn post(&self, url: &str) -> RequestBuilder {
        self.client.post(format!("http://{}{}", self.addr, url))
    }
}

#[tokio::test]
async fn deserialize_body() {
    #[derive(Debug, Deserialize)]
    struct Input {
        #[serde(rename = "@foo")]
        foo: String,
    }

    let app = Router::new().route("/", post(|input: Xml<Input>| async { input.0.foo }));

    let client = TestClient::new(app);
    let res = client
        .await
        .post("/")
        .body(r#"<Input foo="bar"/>"#)
        .header("content-type", "application/xml")
        .send()
        .await
        .unwrap();
    let body = res.text().await.unwrap();

    assert_eq!(body, "bar");
}

#[tokio::test]
async fn consume_body_to_xml_requires_xml_content_type() {
    #[derive(Debug, Deserialize)]
    struct Input {
        #[serde(rename = "@foo")]
        foo: String,
    }

    let app = Router::new().route("/", post(|input: Xml<Input>| async { input.0.foo }));

    let client = TestClient::new(app);
    let res = client
        .await
        .post("/")
        .body(r#"<Input foo="bar"/>"#)
        .send()
        .await
        .unwrap();

    let status = res.status();
    assert!(res.text().await.is_ok());

    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn xml_content_types() {
    async fn valid_xml_content_type(content_type: &str) -> bool {
        #[derive(Deserialize)]
        struct Value {}

        println!("testing {:?}", content_type);

        let app: Router<_> = Router::new().route("/", post(|Xml(_): Xml<Value>| async {}));

        let res = TestClient::new(app)
            .await
            .post("/")
            .header("content-type", content_type)
            .body("<Value />")
            .send()
            .await
            .unwrap();

        res.status() == StatusCode::OK
    }

    assert!(valid_xml_content_type("application/xml").await);
    assert!(valid_xml_content_type("application/xml; charset=utf-8").await);
    assert!(valid_xml_content_type("application/xml;charset=utf-8").await);
    assert!(valid_xml_content_type("application/cloudevents+xml").await);
    assert!(valid_xml_content_type("text/xml").await);
    assert!(!valid_xml_content_type("application/json").await);
}
