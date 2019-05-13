use warp::http::HttpTryFrom;

use warp::http::header::{HeaderValue, InvalidHeaderValue};

pub enum ContentType {
    Plain,
}
impl ToString for ContentType {
    fn to_string(&self) -> String {
        match self {
            &ContentType::Plain => "text/plain; charset=utf-8".to_string(),
        }
    }
}
impl HttpTryFrom<ContentType> for HeaderValue {
    type Error = InvalidHeaderValue;
    fn try_from(content_type: ContentType) -> Result<HeaderValue, InvalidHeaderValue> {
        HeaderValue::from_str(&content_type.to_string())
    }
}
