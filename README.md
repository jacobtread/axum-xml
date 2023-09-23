# axum-xml-up

[![crates.io](https://img.shields.io/crates/v/axum-xml?style=flat-square)](https://crates.io/crates/axum-xml)
[![Documentation](https://img.shields.io/docsrs/axum-xml?style=flat-square)](https://docs.rs/axum-xml)

> Fork of https://github.com/PhotonQuantum/axum-xml updating it to the latest axum 0.6 version

XML extractor for axum.

This crate provides struct `Xml` that can be used to extract typed information from request's body.

Uses [quick-xml](https://github.com/tafia/quick-xml) to deserialize and serialize the payloads

## Features

- `encoding`: support non utf-8 payload

## Request Example

When used as an *Extractor* XML content can be deserialized from the request body into some type that implements `serde::Deserialize`. If the request body cannot be parsed, or it does not contain the `Content-Type: application/xml` header, it will reject the request and return a `400 Bad Request` response.

```rust
use axum::{
    extract,
    routing::post,
    Router,
};
use serde::Deserialize;
use axum_xml::Xml;

#[derive(Deserialize)]
struct CreateUser {
    email: String,
    password: String,
}

async fn create_user(Xml(payload): Xml<CreateUser>) {
    // payload is a `CreateUser`
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/users", post(create_user));
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

## Response Example

When used as a response, it can serialize any type that implements `serde::Serialize` to `XML`, and will automatically set `Content-Type: application/xml` header.

```rust
use axum::{
    extract,
    routing::get,
    Router,
};
use serde::Deserialize;
use axum_xml::Xml;

#[derive(Deserialize)]
struct User {
    id: Uuid,
    username: String,
}

async fn get_user(Path(user_id) : Path<Uuid>) -> Xml<User>  {
    let user = find_user(user_id).await;
    Xml(user)
}

async fn find_user(user_id: Uuid) -> User {
    unimplemented!();
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/users", get(get_user));
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```


## License

MIT