use std::sync::Arc;

use handlebars::Handlebars;

use log::{debug, error};

use serde::Serialize;

use warp::http::response::Builder as ResponseBuilder;
use warp::http::Response;
use warp::{Filter, Rejection, Reply};

use crate::git::GitError;
use crate::warp_helper::ContentType;
use crate::{GitRepository, SmeagolError};

pub struct Smeagol {
    handlebars: Arc<Handlebars>,
}
impl Smeagol {
    pub fn new() -> Result<Smeagol, SmeagolError> {
        debug!("Initializing");
        Ok(Smeagol {
            handlebars: Arc::new(Self::initialize_handlebars()?),
        })
    }
    fn initialize_handlebars() -> Result<Handlebars, SmeagolError> {
        debug!("Initializing Handlebars");
        let mut handlebars = Handlebars::new();
        handlebars.register_templates_directory(".hbs", "templates/")?;

        Ok(handlebars)
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

    fn index(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path::end().map(|| "Hello!")
    }

    fn get(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        #[derive(Serialize)]
        struct TemplateGetData {
            content: String,
        }
        warp::get2()
            .and(
                warp::path::full()
                    .map(|fullpath: warp::filters::path::FullPath| fullpath.as_str().to_string()),
            )
            .and(self.templates())
            .and_then(
                |path: String, templates: Arc<Handlebars>| -> Result<Response<String>, Rejection> {
                    // Remove leading slash
                    let path = GitRepository::parse_path(&path);

                    let repo = GitRepository::new("repo")?;
                    let item = repo.item(path)?;
                    match item.content() {
                        Ok(content) => Ok(ResponseBuilder::new()
                            .header(warp::http::header::CONTENT_TYPE, ContentType::Plain)
                            .status(200)
                            .body(
                                templates
                                    .render(
                                        "get.html",
                                        &TemplateGetData {
                                            content: String::from_utf8_lossy(&content[..])
                                                .to_string(),
                                        },
                                    )
                                    .map_err(|err| SmeagolError::from(err))?,
                            )
                            .unwrap()),
                        Err(GitError::NotFound) => Ok(ResponseBuilder::new()
                            .header(warp::http::header::CONTENT_TYPE, ContentType::Plain)
                            .status(404)
                            .body("Not found.".to_string())
                            .unwrap()),
                        Err(err) => Err(err.into()),
                    }
                },
            )
    }

    fn templates(&self) -> impl Filter<Extract = (Arc<Handlebars>,), Error = Rejection> + Clone {
        let handlebars = self.handlebars.clone();
        warp::any()
            .and_then(move || -> Result<Arc<Handlebars>, Rejection> { Ok(handlebars.clone()) })
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
