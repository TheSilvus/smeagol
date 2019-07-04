use std::cmp::Ordering;
use std::sync::Arc;

use handlebars::Handlebars;

use itertools::Itertools;

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
    /// Initializes the Smeagol instance.
    ///
    /// This reads the config file and initializes the template engine.
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

    /// Starts serving the routes.
    pub fn start(self) -> Result<(), SmeagolError> {
        warp::serve(self.routes()).run(self.config.parse_bind()?);

        Ok(())
    }

    /// Collects the different routes and returns a single Filter.
    fn routes(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        self.statics()
            .or(self.edit().recover(self.handle_500_html()))
            .or(self.post().recover(self.handle_500_json()))
            .or(self.list().recover(self.handle_500_html()))
            .or(self.get().recover(self.handle_500_html()))
            .with(warp::log::log("smeagol"))
    }

    /// Returns an error recovery handler for use with `Filter.recover` in HTML pages.
    ///
    /// If the given route rejects with a `SmeagolError` the rejection is caught and replaced with
    /// a simple error 500 page.
    ///
    /// For JSON endpoints `self.handle_500_json` can be used.
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

    /// Returns an error recovery handler for use with `Filter.recover` in JSON endpoints.
    ///
    /// If the given route rejects with a `SmeagolError` the rejection is caught and replaced with
    /// a simple error 500 value:
    ///
    /// ```json
    /// { "error": "An internal error occurred." }
    /// ```
    ///
    /// For HTML pages `self.handle_500_html` can be used.
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

    /// Serves all files in `./static` under `/static`. Matches only existing files.
    fn statics(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::get2()
            .and(warp::filters::path::path("static"))
            .and(warp::fs::dir("static/"))
    }

    /// Serves an file in the repository. Matches any URL.
    ///
    /// If the file does not exist a 404 page is served. If the file can be created a
    /// corresponding link is shown.
    ///
    /// It used the following steps to determine how the data is presented:
    ///
    /// 1. If a directory is selected a redirect to `config.index` is returned.
    /// 1. If the file only contains valid UTF-8 and the filetype is not raw it is embedded within
    ///    a page for display. `Filetype.parse` is used.
    /// 1. If the filetype is raw and raw inline the file is shown as its own page.
    /// 1. The file is offered for download.
    fn get(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        /// Data for `get.html.hbs`.
        #[derive(Serialize)]
        struct TemplateGetData {
            /// Path of the served file.
            path: String,
            /// Link to list the directory containing the file.
            parent_list_link: String,
            /// Content of the file.
            content: String,
            /// Whether the file content needs to be escaped.
            safe: bool,
        }
        /// Data for `get_not_found.hbs`.
        #[derive(Serialize)]
        struct TemplateGetNotFoundData {
            /// Path of hte served file.
            path: String,
            /// Whether the file can be created.
            can_exist: bool,
        }

        warp::get2()
            .and(
                warp::path::full().map(|fullpath: warp::filters::path::FullPath| {
                    // String conversion is valid because it is still percent encoded.
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
                            // Possible: Get rid of clone?
                            let parsed_utf8 = String::from_utf8(content.clone());

                            // let binding not used because of additional checks
                            if !filetype.is_raw() && parsed_utf8.is_ok() {
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

    /// Serves an edit page for a file in the repository. Matches any URL with `edit` query
    /// parameter. Also allows uploading files.
    ///
    /// It does not matter whether the file exists or not. If it is empty an empty textarea is
    /// shown. If the file cannot exist a special page is served.
    ///
    /// If the file is invalid UTF-8 the textarea is empty and a warning is shown.
    fn edit(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        /// Query parameter matcher.
        ///
        /// The actual value of the query parameter does not matter and is never accessed. It only
        /// matters whether it is given (`?edit`, `?edit=`, `?edit=abc`).
        #[derive(Deserialize)]
        struct QueryParameters {
            // This field is never accessed but is required for the tag
            #[allow(dead_code)]
            edit: String,
        }

        /// Data for `edit.html.hbs`.
        #[derive(Serialize)]
        struct TemplateEditData<'a> {
            /// Path of the edited file.
            path: String,
            /// Content of the edited file. It is empty and ignored if `is_valid` is false.
            content: String,
            /// Whether the content of the file is valid UTF-8.
            ///
            /// If not, a warning is shown above the text area.
            is_valid: bool,
            // TODO replace config with used parameters
            config: &'a Config,
        }
        /// Data for `edit_cannot_exist.html.hbs`.
        #[derive(Serialize)]
        struct TemplateCannotExistData {
            /// Path of the edited file.
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

    /// Edits or creates a file in the repository. Matches any URL.
    ///
    /// Requires a query paramater `commit_message`. File content is in request body.
    fn post(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        /// Query paramter matcher.
        #[derive(Deserialize)]
        struct QueryParameters {
            commit_message: String,
        }
        /// Data returned if the edit was successful.
        ///
        /// Contains the path of the edited/created file.
        #[derive(Serialize)]
        struct EditSuccessData {
            path: String,
        }
        /// Data returned if an error occurred during the edit.
        ///
        /// Contains an error message.
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
    /// Serves a page listing all files in a directory. Matches any URL with a `list` query
    /// paramter.
    ///
    /// If the directory does not exist a 404 page is served. If the path points to a file a
    /// redirect is added.
    ///
    /// Directories are sorted first.
    fn list(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        /// Query parameter matcher.
        ///
        /// The actual value of the query parameter does not matter and is never accessed. It only
        /// matters whether it is given (`?list`, `?list=`, `?list=abc`).
        #[derive(Deserialize)]
        struct QueryParameters {
            // This field is never accessed but is required for the tag
            #[allow(dead_code)]
            list: String,
        }
        /// Data for `list.html.hbs`.
        #[derive(Serialize)]
        struct TemplateListData {
            /// Path of the listed directory.
            path: String,
            /// Link to the parent directory if it exist (the listed directory is not the root).
            parent_list_link: Option<String>,
            /// List of all the items in the directory.
            children: Vec<TemplateListChildData>,
        }
        /// A list item.
        #[derive(Serialize)]
        struct TemplateListChildData {
            /// Link to the item.
            link: String,
            /// Name of the item.
            name: String,
        }
        /// Data for `list_not_found.html.hbs`.
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
                                    .sorted_by(|a, b| {
                                        // Error handling instead of unwrap would be preferable but
                                        // horrible in this situation. There should be no errors
                                        // anyways; the files have to exist (the only reasonable
                                        // error source).
                                        if a.is_dir().unwrap() && b.is_file().unwrap() {
                                            Ordering::Less
                                        } else if a.is_file().unwrap() && b.is_dir().unwrap() {
                                            Ordering::Greater
                                        } else {
                                            // Unwrap because the root cannot be one of the listed
                                            // items.
                                            a.path()
                                                .filename()
                                                .unwrap()
                                                .bytes()
                                                .cmp(b.path().filename().unwrap().bytes())
                                        }
                                    })
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
                                    // A list of results can be collected to a result of a list.
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

    /// Returns a filter that returns the template handler for use with `.and`.
    fn templates(&self) -> impl Filter<Extract = (Arc<Handlebars>,), Error = Rejection> + Clone {
        let handlebars = self.handlebars.clone();
        warp::any()
            .and_then(move || -> Result<Arc<Handlebars>, Rejection> { Ok(handlebars.clone()) })
    }
    /// Returns a filter that returns the config for use with `.and`.
    fn config(&self) -> impl Filter<Extract = (Arc<Config>,), Error = Rejection> + Clone {
        let config = self.config.clone();
        warp::any().and_then(move || -> Result<Arc<Config>, Rejection> { Ok(config.clone()) })
    }
}
