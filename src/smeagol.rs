use log::{debug, error, info};

use warp::http::response::Builder as ResponseBuilder;
use warp::http::Response;
use warp::{Filter, Rejection, Reply};

use crate::git::GitError;
use crate::warp_helper::ContentType;
use crate::{GitRepository, SmeagolError};

pub struct Smeagol {}
impl Smeagol {
    pub fn new() -> Smeagol {
        debug!("Initializing");
        Smeagol {}
    }

    pub fn start(self) {
        debug!("Starting on 127.0.0.1:8000");

        warp::serve(self.routes()).run(([127, 0, 0, 1], 8000));
    }

    fn routes(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        self.index()
            .or(self.get())
            .with(warp::log::log("smeagol"))
            .recover(Self::recover_500())
    }

    fn index(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path::end().map(|| "Hello!")
    }

    fn get(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::get2()
            .and(
                warp::path::full()
                    .map(|fullpath: warp::filters::path::FullPath| fullpath.as_str().to_string()),
            )
            .and_then(|path: String| -> Result<Response<String>, Rejection> {
                // Remove leading slash
                let path = GitRepository::parse_path(&path);

                let repo = GitRepository::new("repo")?;
                let item = repo.item(path)?;
                match item.content() {
                    Ok(content) => Ok(ResponseBuilder::new()
                        .header(warp::http::header::CONTENT_TYPE, ContentType::Plain)
                        .status(200)
                        .body(String::from_utf8_lossy(&content[..]).to_string())
                        .unwrap()),
                    Err(GitError::NotFound) => Ok(ResponseBuilder::new()
                        .header(warp::http::header::CONTENT_TYPE, ContentType::Plain)
                        .status(404)
                        .body("Not found.".to_string())
                        .unwrap()),
                    Err(err) => Err(err.into()),
                }
            })
    }

    fn recover_500(
    ) -> impl Fn(warp::Rejection) -> Result<warp::http::Response<String>, Rejection> + Clone {
        |err: warp::Rejection| {
            if let Some(ref err) = err.find_cause::<SmeagolError>() {
                error!("Internal error: {}", err);
                Ok(ResponseBuilder::new()
                    .header(warp::http::header::CONTENT_TYPE, ContentType::Plain)
                    .status(500)
                    .body("An internal error occurred.".to_string())
                    .unwrap())
            } else {
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_index() {
        let smeagol = crate::Smeagol::new();
        let res = warp::test::request().path("/").reply(&smeagol.index());

        assert_eq!(res.status(), 200);
        assert_eq!(res.body(), "Hello!");
    }
}
