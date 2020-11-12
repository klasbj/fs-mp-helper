#[tokio::main]
async fn main() {
    let db = models::blank_db();
    let api = filters::create_api(db);

    warp::serve(api).run(([127, 0, 0, 1], 3030)).await;
}

mod filters {
    use super::handlers;
    use super::models::Db;
    use warp::Filter;

    pub fn create_api(
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_settings(db.clone())
            .or(set_settings(db.clone()))
            .or(get_aircraft(db.clone()))
            .or(set_aircraft(db.clone()))
    }

    fn get_settings(
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("settings")
            .and(warp::get())
            .and(with_db(db))
            .and_then(handlers::get_settings)
    }

    fn set_settings(
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("settings")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_db(db))
            .and_then(handlers::set_settings)
    }

    fn get_aircraft(
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("aircraft")
            .and(warp::get())
            .and(with_db(db))
            .and_then(handlers::get_aircraft)
    }

    fn set_aircraft(
        db: Db,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("aircraft")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_db(db))
            .and_then(handlers::set_aircraft)
    }

    fn with_db(db: Db) -> impl Filter<Extract = (Db,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || db.clone())
    }
}

mod handlers {
    use super::models::{AircraftData, Db, Settings};
    use std::convert::Infallible;
    use std::time::{Duration, Instant};

    pub async fn get_settings(db: Db) -> Result<impl warp::Reply, Infallible> {
        let state = db.lock().await;
        Ok(warp::reply::json(&state.settings))
    }

    pub async fn set_settings(settings: Settings, db: Db) -> Result<impl warp::Reply, Infallible> {
        let mut state = db.lock().await;
        state.settings = settings;
        Ok(warp::reply::json(&state.settings))
    }

    pub async fn get_aircraft(db: Db) -> Result<impl warp::Reply, Infallible> {
        let state = db.lock().await;

        let oldest_allowed = Instant::now() - Duration::from_secs(60 * 10);
        let result: Vec<AircraftData> = state
            .aircraft
            .iter()
            .filter(|(_, t)| t > &oldest_allowed)
            .map(|(ac, _)| ac.clone())
            .collect();

        Ok(warp::reply::json(&result))
    }

    pub async fn set_aircraft(
        updated_ac: AircraftData,
        db: Db,
    ) -> Result<impl warp::Reply, Infallible> {
        let mut state = db.lock().await;

        let now = Instant::now();

        match state
            .aircraft
            .iter()
            .position(|(ac, _)| ac.name == updated_ac.name)
        {
            Some(i) => state.aircraft[i] = (updated_ac, now),
            None => state.aircraft.push((updated_ac, now)),
        };

        Ok(warp::reply::json(&state.settings))
    }
}

mod models {
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::Mutex;
    pub type Db = Arc<Mutex<State>>;

    pub fn blank_db() -> Db {
        Arc::new(Mutex::new(State {
            settings: Settings { show_tags: true },
            aircraft: Vec::new(),
        }))
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct AircraftData {
        pub name: String,
        pub latitude: f64,
        pub longitude: f64,
        pub altitude: f64,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct Settings {
        pub show_tags: bool,
    }

    pub struct State {
        pub settings: Settings,
        pub aircraft: Vec<(AircraftData, Instant)>,
    }
}
