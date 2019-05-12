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
        return self.index().or(self.something());
    }

    fn index(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path::end().map(|| "Hello!")
    }

    fn something(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path::path("test")
            .and(warp::path::end())
            .map(|| "Hello!")
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
