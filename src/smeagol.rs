use log::{debug, error, info};

use warp::{Filter, Reply};

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
        self.index().or(self.get()).with(warp::log::log("smeagol"))
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
            .and_then(|path: String| -> Result<String, warp::Rejection> {
                // Remove leading slash
                let path = Self::parse_path(&path[1..]);

                let repo = crate::GitRepository::new("repo")?;
                let item = repo.item(path)?;
                match item.content() {
                    Ok(content) => Ok(String::from_utf8_lossy(&content[..]).to_string()),
                    Err(crate::git::GitError::NotFound) => Ok("Not found".to_string()),
                    Err(err) => Err(err.into()),
                }
            })
            .recover(Self::recover_500())
    }

    fn recover_500() -> impl Fn(warp::Rejection) -> Result<String, warp::Rejection> + Clone {
        |err: warp::Rejection| {
            if let Some(ref err) = err.find_cause::<crate::SmeagolError>() {
                error!("Internal error: {}", err);
                Ok("An internal error occurred.".to_string())
            } else {
                Err(err)
            }
        }
    }

    // TODO check: can I use borrowed paths?
    fn parse_path(path: &str) -> Vec<Vec<u8>> {
        path.split("/")
            .map(|s| s.bytes().collect::<Vec<_>>())
            .collect::<Vec<_>>()
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
