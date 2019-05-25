use std::sync::Arc;

use handlebars::Handlebars;

use log::{debug, error};

use serde::{Deserialize, Serialize};

use warp::http::Response;
use warp::{Buf, Filter, Rejection, Reply};

use crate::git::GitError;
use crate::warp_helper::ResponseBuilder;
use crate::{Config, Filetype, GitRepository, Path, PathStringBuilder, SmeagolError};

pub struct Smeagol {
    handlebars: Arc<Handlebars>,
    config: Arc<Config>,
}
impl Smeagol {
    pub fn new() -> Result<Smeagol, SmeagolError> {
        debug!("Initializing");

        let config_file = std::env::var("SMEAGOL_CONF").unwrap_or("Smeagol.toml".to_string());

        Ok(Smeagol {
            handlebars: Arc::new(Self::initialize_handlebars()?),
            config: Arc::new(Config::load(&config_file)?),
        })
    }
    fn initialize_handlebars() -> Result<Handlebars, SmeagolError> {
        debug!("Initializing Handlebars");
        let mut handlebars = Handlebars::new();
        handlebars.register_templates_directory(".hbs", "templates/")?;

        Ok(handlebars)
    }

    pub fn start(self) -> Result<(), SmeagolError> {
        warp::serve(self.routes()).run(self.config.parse_bind()?);

        Ok(())
    }

    fn routes(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        self.statics()
            .or(self.edit().recover(self.handle_500_html()))
            .or(self.edit_post().recover(self.handle_500_json()))
            .or(self.list().recover(self.handle_500_html()))
            .or(self.get().recover(self.handle_500_html()))
            .with(warp::log::log("smeagol"))
    }

    fn handle_500_html(
        &self,
    ) -> impl Fn(warp::Rejection) -> Result<warp::http::Response<Vec<u8>>, Rejection> + Clone {
        let templates = self.handlebars.clone();

        #[derive(Serialize)]
        struct Template500Data {}
        move |err: warp::Rejection| {
            if let Some(ref err) = err.find_cause::<SmeagolError>() {
                error!("Internal error: {}", err);
                Ok(ResponseBuilder::new().status(500).body_template(
                    &templates,
                    "500.html",
                    &Template500Data {},
                )?)
            } else {
                Err(err)
            }
        }
    }
    fn handle_500_json(
        &self,
    ) -> impl Fn(warp::Rejection) -> Result<warp::http::Response<Vec<u8>>, Rejection> + Clone {
        #[derive(Serialize)]
        struct Json500Error {
            error: String,
        }
        |err: warp::Rejection| {
            if let Some(ref err) = err.find_cause::<SmeagolError>() {
                error!("Internal error: {}", err);
                Ok(ResponseBuilder::new()
                    .status(500)
                    .body_json(&Json500Error {
                        error: "An internal error occurred.".to_string(),
                    })?)
            } else {
                Err(err)
            }
        }
    }

    fn statics(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::get2()
            .and(warp::filters::path::path("static"))
            .and(warp::fs::dir("static/"))
    }

    fn get(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        #[derive(Serialize)]
        struct TemplateGetData {
            path: String,
            parent_list_link: String,
            content: String,
            safe: bool,
        }
        #[derive(Serialize)]
        struct TemplateGetNotFoundData {
            path: String,
            can_exist: bool,
        }
        warp::get2()
            .and(
                warp::path::full().map(|fullpath: warp::filters::path::FullPath| {
                    Path::from_percent_encoded(fullpath.as_str().to_string().as_bytes())
                }),
            )
            .and(self.templates())
            .and(self.config())
            .and_then(
                |path: Path,
                 templates: Arc<Handlebars>,
                 config: Arc<Config>|
                 -> Result<Response<Vec<u8>>, Rejection> {
                    let repo = GitRepository::new(&config.repo)?;
                    let item = repo.item(path.clone())?;

                    match item.content() {
                        Ok(content) => {
                            let filetype = Filetype::from(&path);
                            let raw = filetype.is_raw();
                            // Possible: Get rid of clone
                            let parsed_utf8 = String::from_utf8(content.clone());

                            if !raw && parsed_utf8.is_ok() {
                                Ok(ResponseBuilder::new().status(200).body_template(
                                    &templates,
                                    "get.html",
                                    &TemplateGetData {
                                        path: path.to_string(),
                                        // File path has to have parent
                                        parent_list_link: format!(
                                            "{}?list",
                                            PathStringBuilder::new(path.parent().unwrap(),)
                                                .root(true)
                                                .build_percent_encode()
                                        ),
                                        content: filetype
                                            .parse(
                                                // parsing result checked above
                                                &parsed_utf8.unwrap(),
                                            )
                                            .map_err(|err| SmeagolError::from(err))?,
                                        safe: filetype.is_safe(),
                                    },
                                )?)
                            } else {
                                if filetype.is_raw() && filetype.is_raw_inline() {
                                    Ok(ResponseBuilder::new()
                                        .header(
                                            warp::http::header::CONTENT_TYPE,
                                            filetype.content_type(),
                                        )
                                        .status(200)
                                        .body(content))
                                } else {
                                    Ok(ResponseBuilder::new()
                                        .header(
                                            warp::http::header::CONTENT_TYPE,
                                            filetype.content_type(),
                                        )
                                        .status(200)
                                        .body_download(content))
                                }
                            }
                        }
                        Err(GitError::IsDir) => {
                            let mut redirect_path = path;
                            redirect_path.push(config.index.to_string());
                            Ok(ResponseBuilder::new().redirect(redirect_path))
                        }
                        Err(GitError::NotFound) => {
                            Ok(ResponseBuilder::new().status(404).body_template(
                                &templates,
                                "get_not_found.html",
                                &TemplateGetNotFoundData {
                                    path: path.to_string(),
                                    can_exist: item.can_exist()?,
                                },
                            )?)
                        }
                        Err(err) => Err(err.into()),
                    }
                },
            )
    }

    fn edit(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        #[derive(Deserialize)]
        struct QueryParameters {
            // This field is never accessed but is required for the tag
            #[allow(dead_code)]
            edit: String,
        }
        #[derive(Serialize)]
        struct TemplateCannotExistData {
            path: String,
        }
        #[derive(Serialize)]
        struct TemplateEditData<'a> {
            path: String,
            content: String,
            is_valid: bool,
            config: &'a Config,
        }
        warp::get2()
            .and(
                warp::path::full().map(|fullpath: warp::filters::path::FullPath| {
                    Path::from_percent_encoded(fullpath.as_str().to_string().as_bytes())
                }),
            )
            .and(warp::query::<QueryParameters>())
            .and(self.templates())
            .and(self.config())
            .and_then(
                |path: Path,
                 _: QueryParameters,
                 templates: Arc<Handlebars>,
                 config: Arc<Config>|
                 -> Result<Response<Vec<u8>>, Rejection> {
                    let repo = GitRepository::new(&config.repo)?;
                    let item = repo.item(path.clone())?;

                    if !item.can_exist()? || (item.exists()? && item.is_dir()?) {
                        return Ok(ResponseBuilder::new().status(400).body_template(
                            &templates,
                            "edit_cannot_exist.html",
                            &TemplateCannotExistData {
                                path: path.to_string(),
                            },
                        )?);
                    }

                    match item.content() {
                        Ok(content) => {
                            let parsed_content = String::from_utf8(content);
                            Ok(ResponseBuilder::new().status(200).body_template(
                                &templates,
                                "edit.html",
                                &TemplateEditData {
                                    path: path.to_string(),
                                    is_valid: parsed_content.is_ok(),
                                    content: parsed_content.unwrap_or("".to_string()),
                                    config: &config,
                                },
                            )?)
                        }
                        Err(GitError::NotFound) => {
                            Ok(ResponseBuilder::new().status(200).body_template(
                                &templates,
                                "edit.html",
                                &TemplateEditData {
                                    path: path.to_string(),
                                    is_valid: true,
                                    content: "".to_string(),
                                    config: &config,
                                },
                            )?)
                        }
                        Err(err) => Err(err.into()),
                    }
                },
            )
    }

    fn edit_post(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        #[derive(Deserialize)]
        struct QueryParameters {
            commit_message: String,
        }
        #[derive(Serialize)]
        struct EditSuccessData {
            path: String,
        }
        #[derive(Serialize)]
        struct EditErrorData {
            error: String,
        }
        warp::post2()
            .and(
                warp::path::full().map(|fullpath: warp::filters::path::FullPath| {
                    Path::from_percent_encoded(fullpath.as_str().to_string().as_bytes())
                }),
            )
            .and(warp::query::<QueryParameters>())
            .and(self.config())
            .and(
                warp::body::content_length_limit(self.config.max_upload_size)
                    .and(warp::body::concat()),
            )
            .and_then(
                |path: Path,
                 query: QueryParameters,
                 config: Arc<Config>,
                 mut body: warp::body::FullBody|
                 -> Result<Response<Vec<u8>>, Rejection> {
                    let mut buffer = vec![0; body.remaining()];
                    body.copy_to_slice(&mut buffer[..]);

                    let repo = GitRepository::new(&config.repo)?;
                    let item = repo.item(path.clone())?;

                    match item.edit(&buffer[..], &query.commit_message) {
                        Ok(()) | Err(GitError::NoChange) => Ok(ResponseBuilder::new()
                            .status(200)
                            .body_json(&EditSuccessData {
                                path: PathStringBuilder::new(path)
                                    .root(true)
                                    .build_percent_encode(),
                            })?),
                        Err(GitError::CannotCreate) => Ok(ResponseBuilder::new()
                            .status(400)
                            .body_json(&EditErrorData {
                                error: "Could not create file at that location.".to_string(),
                            })?),
                        Err(err) => Err(err.into()),
                    }
                },
            )
    }
    fn list(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        #[derive(Deserialize)]
        struct QueryParameters {
            // This field is never accessed but is required for the tag
            #[allow(dead_code)]
            list: String,
        }
        #[derive(Serialize)]
        struct TemplateListData {
            path: String,
            parent_list_link: Option<String>,
            children: Vec<TemplateListChildData>,
        }
        #[derive(Serialize)]
        struct TemplateListChildData {
            link: String,
            name: String,
        }
        #[derive(Serialize)]
        struct TemplateListNotFoundData {
            path: String,
        }
        warp::get2()
            .and(
                warp::path::full().map(|fullpath: warp::filters::path::FullPath| {
                    Path::from_percent_encoded(fullpath.as_str().to_string().as_bytes())
                }),
            )
            .and(warp::query::<QueryParameters>())
            .and(self.templates())
            .and(self.config())
            .and_then(
                |path: Path,
                 _: QueryParameters,
                 templates: Arc<Handlebars>,
                 config: Arc<Config>|
                 -> Result<Response<Vec<u8>>, Rejection> {
                    let repo = GitRepository::new(&config.repo)?;
                    let item = repo.item(path.clone())?;

                    match item.list() {
                        Ok(items) => Ok(ResponseBuilder::new().status(200).body_template(
                            &templates,
                            "list.html",
                            &TemplateListData {
                                path: PathStringBuilder::new(path.clone()).dir(true).build_lossy(),
                                parent_list_link: path.clone().parent().map(|path| {
                                    format!(
                                        "{}?list",
                                        PathStringBuilder::new(path)
                                            .root(true)
                                            .build_percent_encode()
                                    )
                                }),
                                children: items
                                    .iter()
                                    .map(|item| -> Result<TemplateListChildData, GitError> {
                                        let link = if item.is_dir()? {
                                            format!(
                                                "{}?list",
                                                PathStringBuilder::new(item.path().clone())
                                                    .root(true)
                                                    .build_percent_encode()
                                            )
                                        } else {
                                            PathStringBuilder::new(item.path().clone())
                                                .root(true)
                                                .build_percent_encode()
                                        };
                                        Ok(TemplateListChildData {
                                            link: link,
                                            name: PathStringBuilder::new(
                                                // A child of something has to have a filename
                                                item.path().filename().unwrap(),
                                            )
                                            .dir(item.is_dir()?)
                                            .build_lossy(),
                                        })
                                    })
                                    .collect::<Result<Vec<_>, _>>()?,
                            },
                        )?),
                        Err(GitError::NotFound) => {
                            Ok(ResponseBuilder::new().status(200).body_template(
                                &templates,
                                "list_not_found.html",
                                &TemplateListNotFoundData {
                                    path: PathStringBuilder::new(path).dir(true).build_lossy(),
                                },
                            )?)
                        }
                        Err(GitError::IsFile) => Ok(ResponseBuilder::new().redirect(path)),
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
    fn config(&self) -> impl Filter<Extract = (Arc<Config>,), Error = Rejection> + Clone {
        let config = self.config.clone();
        warp::any().and_then(move || -> Result<Arc<Config>, Rejection> { Ok(config.clone()) })
    }
}
