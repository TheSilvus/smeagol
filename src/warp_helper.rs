use std::fmt;

use handlebars::Handlebars;

use serde::Serialize;

use warp::http::header::{HeaderName, HeaderValue, InvalidHeaderValue};
use warp::http::response::Builder as HttpResponseBuilder;
use warp::http::status::StatusCode;
use warp::http::{HttpTryFrom, Response};

use crate::SmeagolError;

pub enum ContentType {
    Plain,
    Html,
    Json,
}
impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ContentType::Plain => write!(f, "text/plain; charset=utf-8"),
            &ContentType::Html => write!(f, "text/html; charset=utf-8"),
            &ContentType::Json => write!(f, "application/json"),
        }
    }
}
impl HttpTryFrom<ContentType> for HeaderValue {
    type Error = InvalidHeaderValue;
    fn try_from(content_type: ContentType) -> Result<HeaderValue, InvalidHeaderValue> {
        HeaderValue::from_str(&content_type.to_string())
    }
}

pub struct ResponseBuilder {
    builder: HttpResponseBuilder,
}
impl ResponseBuilder {
    pub fn new() -> ResponseBuilder {
        ResponseBuilder {
            builder: HttpResponseBuilder::new(),
        }
    }

    pub fn header<K, V>(&mut self, key: K, value: V) -> &mut ResponseBuilder
    where
        HeaderName: HttpTryFrom<K>,
        HeaderValue: HttpTryFrom<V>,
    {
        self.builder.header(key, value);
        self
    }

    pub fn status<T>(&mut self, status: T) -> &mut ResponseBuilder
    where
        StatusCode: HttpTryFrom<T>,
    {
        self.builder.status(status);
        self
    }

    pub fn body<T>(&mut self, body: T) -> Response<T> {
        // ResponseBuilder.body() cannot return Err(...) currently (checked in code). This may
        // change in the future though.
        self.builder.body(body).unwrap()
    }

    pub fn body_template<T: Serialize>(
        &mut self,
        templates: &Handlebars,
        template: &str,
        data: &T,
    ) -> Result<Response<String>, SmeagolError> {
        Ok(self.body(
            templates
                .render(template, data)
                .map_err(|err| SmeagolError::from(err))?,
        ))
    }

    pub fn body_json<T: Serialize>(&mut self, data: &T) -> Result<Response<String>, SmeagolError> {
        Ok(self
            .header(warp::http::header::CONTENT_TYPE, ContentType::Json)
            .body(serde_json::to_string(data)?))
    }
}
