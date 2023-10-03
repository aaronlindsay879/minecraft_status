mod config;

use anyhow::Result;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use config::Config;
use gamedig::protocols::minecraft::JavaResponse;
use log::{debug, info, warn, LevelFilter};
use minijinja::render;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};

const DEFAULT_PORT: u16 = 3000;

type Shared<T> = Arc<RwLock<T>>;

#[tokio::main]
async fn main() -> Result<()> {
    // read env file and init logger with default warn level
    let _ = dotenvy::dotenv();
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Warn)
        .env()
        .init()
        .unwrap();

    // create shared server status
    let status = Arc::new(RwLock::new(None));

    // set up background process to refresh server status
    let config = Config::from_env_vars()?;
    info!("using config {config:?}");

    let status_clone = status.clone();
    tokio::task::spawn_blocking(move || loop {
        update_status(&status_clone, &config.ip, config.port.clone());
        std::thread::sleep(config.refresh_interval);
    });

    // create router
    let app = Router::new().route("/", get(move || serve_status(status)));
    // find port to run server on
    let port = get_port();

    info!("listening on 0.0.0.0:{port}");
    axum::Server::bind(&format!("0.0.0.0:{port}").parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

/// Finds port to run server on
fn get_port() -> u16 {
    let port_string = std::env::var("PORT");

    match port_string {
        Ok(port_string) => match port_string.parse() {
            Ok(port) => port,
            Err(_) => {
                warn!("env var `PORT` has invalid value `{port_string}`");
                DEFAULT_PORT
            }
        },
        _ => DEFAULT_PORT,
    }
}

/// Updates a status with result from given ip/port
fn update_status(status: &Shared<Option<JavaResponse>>, ip: &IpAddr, port: u16) {
    // get status, log and then write
    let new_status = gamedig::games::mc::query(ip, Some(port)).ok();
    debug!("status:\n{new_status:?}");

    let mut write = status.write().unwrap();
    *write = new_status;
}

/// Serves the status, returning a different page depending on if the server is up (Some) or down (None)
async fn serve_status(status: Shared<Option<JavaResponse>>) -> Html<String> {
    const SERVER_UP_STATUS: &'static str = include_str!("../templates/server_up.html");
    const SERVER_DOWN_STATUS: &'static str = include_str!("../templates/server_down.html");

    let read = (*status.read().unwrap()).clone();
    let hostname = std::env::var("SERVER").unwrap();

    Html(match read {
        Some(response) => {
            info!("serving up status");
            render!(SERVER_UP_STATUS, status => response, server => hostname)
        }
        None => {
            info!("serving down status");
            render!(SERVER_DOWN_STATUS, server => hostname)
        }
    })
}
