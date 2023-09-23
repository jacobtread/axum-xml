//! XML extractor for axum
//!
//! This crate provides struct `Xml` that can be used to extract typed information from request's body.
//!
//! Under the hood, [quick-xml](https://github.com/tafia/quick-xml) is used to parse payloads.
//!
//! ## Features
//!
//! - `encoding`: support non utf-8 payload
#![allow(clippy::module_name_repetitions)]

use std::ops::{Deref, DerefMut};

use axum_core::extract::FromRequest;
use axum_core::response::{IntoResponse, Response};
use axum_core::BoxError;
use bytes::Bytes;
use http::{header, HeaderValue, Request, StatusCode};
use http_body::Body as HttpBody;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::rejection::XmlRejection;

mod rejection;
#[cfg(test)]
mod tests;

/// XML Extractor / Response.
///
/// When used as an extractor, it can deserialize request bodies into some type that
/// implements [`serde::Deserialize`]. If the request body cannot be parsed, or it does not contain
/// the `Content-Type: application/xml` header, it will reject the request and return a
/// `400 Bad Request` response.
///
/// # Extractor example
///
/// ```rust,no_run
/// use axum::{
///     extract,
///     routing::post,
///     Router,
/// };
/// use serde::Deserialize;
/// use axum_xml::Xml;
///
/// #[derive(Deserialize)]
/// struct CreateUser {
///     email: String,
///     password: String,
/// }
///
/// async fn create_user(Xml(payload): Xml<CreateUser>) {
///     // payload is a `CreateUser`
/// }
///
/// let app = Router::new().route("/users", post(create_user));
/// # async {
/// # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
/// # };
/// ```
///
/// When used as a response, it can serialize any type that implements [`serde::Serialize`] to
/// `XML`, and will automatically set `Content-Type: application/xml` header.
///
/// # Response example
///
/// ```
/// use axum::{
///     extract::Path,
///     routing::get,
///     Router,
/// };
/// use serde::Serialize;
/// use uuid::Uuid;
/// use axum_xml::Xml;
///
/// #[derive(Serialize)]
/// struct User {
///     id: Uuid,
///     username: String,
/// }
///
/// async fn get_user(Path(user_id) : Path<Uuid>) -> Xml<User> {
///     let user = find_user(user_id).await;
///     Xml(user)
/// }
///
/// async fn find_user(user_id: Uuid) -> User {
///     // ...
///     # unimplemented!()
/// }
///
/// let app = Router::new().route("/users/:id", get(get_user));
/// # async {
/// # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
/// # };
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Xml<T>(pub T);

impl<T, S, B> FromRequest<S, B> for Xml<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = XmlRejection;

    fn from_request<'life0, 'async_trait>(
        req: Request<B>,
        state: &'life0 S,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Self, Self::Rejection>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let content_type = content_type(&req);
            if !content_type.is_some_and(is_xml_type) {
                return Err(XmlRejection::MissingXMLContentType);
            }

            let bytes = Bytes::from_request(req, state).await?;
            let value = quick_xml::de::from_reader(&*bytes)?;

            Ok(Self(value))
        })
    }
}

/// Obtains and parses the mime type of the Content-Type header
fn content_type<B>(req: &Request<B>) -> Option<mime::Mime> {
    req.headers()
        // Get content type header
        .get(header::CONTENT_TYPE)
        // Get the header string value
        .and_then(|value| value.to_str().ok())
        // Parse the mime type
        .and_then(|value| value.parse::<mime::Mime>().ok())
}

/// Checks whether the provided mime type can be considered xml
fn is_xml_type(mime: mime::Mime) -> bool {
    let type_ = mime.type_();
    // Ensure the main type is application/ or text/
    (type_ == "application" || type_ == "text")
    // Ensure the subtype or suffix is xml
        && (mime.subtype() == "xml" || mime.suffix().is_some_and(|value| value == "xml"))
}

impl<T> Deref for Xml<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Xml<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for Xml<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> IntoResponse for Xml<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let mut bytes = Vec::new();
        match quick_xml::se::to_writer(&mut bytes, &self.0) {
            Ok(_) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/xml"),
                )],
                bytes,
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}
