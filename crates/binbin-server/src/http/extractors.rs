//! Keep Axum's extraction status codes while using the API error envelope.
use super::Error;
use axum::{
    body::Bytes,
    extract::{FromRequest, FromRequestParts, Request},
    http::request::Parts,
    response::{IntoResponse, Response},
};
use serde::{Serialize, de::DeserializeOwned};

pub(super) struct Json<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = Error;

    async fn from_request(request: Request, state: &S) -> Result<Self, Error> {
        axum::Json::<T>::from_request(request, state)
            .await
            .map(|axum::Json(value)| Self(value))
            .map_err(|rejection| Error(rejection.status().as_u16(), rejection.body_text()))
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

pub(super) struct Query<T>(pub T);

impl<S, T> FromRequestParts<S> for Query<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Query(value)| Self(value))
            .map_err(|rejection| Error(rejection.status().as_u16(), rejection.body_text()))
    }
}

pub(super) struct BodyBytes(pub Bytes);

impl<S: Send + Sync> FromRequest<S> for BodyBytes {
    type Rejection = Error;

    async fn from_request(request: Request, state: &S) -> Result<Self, Error> {
        Bytes::from_request(request, state)
            .await
            .map(Self)
            .map_err(|rejection| Error(rejection.status().as_u16(), rejection.body_text()))
    }
}
