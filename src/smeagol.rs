use log::{debug, info};
use warp::{Filter, Reply};

pub struct Smeagol {}
impl Smeagol {
    pub fn new() -> Smeagol {
        debug!("Initializing");
        Smeagol {}
    }

    pub fn start(self) {
        info!("Starting on 127.0.0.1:8000");

        warp::serve(self.routes()).run(([127, 0, 0, 1], 8000));
    }

    fn routes(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        return self.index().or(self.get());
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
            .map(|path: String| {
                // Remove leading slash
                let path = Self::parse_path(&path[1..]);

                let repo = crate::GitRepository::new("repo").unwrap();
                let item = repo.item(path).unwrap();
                let content = item.content().unwrap();

                String::from_utf8_lossy(&content[..]).to_string()
            })
    }

    // TODO check: can I use borrowed paths like this everywhere?
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
