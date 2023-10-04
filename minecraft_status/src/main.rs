mod config;

use crate::config::Server;
use anyhow::Result;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use config::Config;
use gamedig::games::mc;
use gamedig::protocols::minecraft::JavaResponse;
use log::{debug, info, warn, LevelFilter};
use minijinja::render;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

const DEFAULT_PORT: u16 = 3000;

type Status = Arc<RwLock<HashMap<String, Option<JavaResponse>>>>;

#[tokio::main]
async fn main() -> Result<()> {
    // read env file and init logger with default warn level
    let _ = dotenvy::dotenv();
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Warn)
        .env()
        .init()
        .unwrap();

    let config = Config::from_env_vars()?;
    info!("using config {config:?}");

    // create shared server status and fill with servers from config
    let status = Arc::new(RwLock::new(
        config
            .servers
            .iter()
            .map(|server| (server.server.clone(), None))
            .collect(),
    ));

    // set up background process to refresh each server status
    for server in &config.servers {
        let server = server.clone();
        let status_clone = status.clone();

        tokio::task::spawn_blocking(move || loop {
            update_status(&status_clone, &server);
            std::thread::sleep(config.refresh_interval);
        });
    }

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

/// Updates a status with result from given server
fn update_status(status: &Status, server: &Server) {
    // get new status, trying java and then bedrock
    let new_status = if let Ok(response) = mc::query_java(&server.ip, Some(server.port)) {
        Some(response)
    } else if let Ok(response) = mc::query_bedrock(&server.ip, Some(server.port)) {
        Some(JavaResponse::from_bedrock_response(response))
    } else {
        None
    };

    // then log and write to shared status
    debug!("status for `{}`:\n\t{new_status:?}", server.server);

    status
        .write()
        .unwrap()
        .insert(server.server.clone(), new_status);
}

/// Serves the status, returning a different page depending on if the server is up (Some) or down (None)
async fn serve_status(status: Status) -> Html<String> {
    const SERVER_UP_STATUS: &'static str = include_str!("../templates/server_up.html");
    const SERVER_DOWN_STATUS: &'static str = include_str!("../templates/server_down.html");

    let read = (*status.read().unwrap()).clone();

    // temp hack, just show first server
    Html(match read.iter().next() {
        Some((hostname, Some(response))) => {
            info!("serving up status");
            render!(SERVER_UP_STATUS, status => response, server => hostname)
        }
        Some((hostname, None)) => {
            info!("serving down status");
            render!(SERVER_DOWN_STATUS, server => hostname)
        }
        _ => {
            warn!("unknown status");
            render!(SERVER_DOWN_STATUS, server => "")
        }
    })
}
