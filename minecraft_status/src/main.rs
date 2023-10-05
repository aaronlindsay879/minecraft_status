#![deny(unsafe_code)]

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
    let router_status = status.clone();
    let mut router = Router::new().route("/", get(move || serve_all_status(router_status)));

    // then add routes for each server
    for server in config.servers.clone() {
        let status = status.clone();

        router = router.route(
            &format!("/{}", server.server),
            get(move || serve_single_status(server.clone().server, status)),
        )
    }

    // find port to run server on
    let port = get_port();

    info!("listening on 0.0.0.0:{port}");
    axum::Server::bind(&format!("0.0.0.0:{port}").parse()?)
        .serve(router.into_make_service())
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

/// Serves the status of all servers
async fn serve_all_status(status: Status) -> Html<String> {
    const SERVE_ALL_STATUS: &'static str = include_str!("../templates/all.html");

    let read = (*status.read().unwrap()).clone();

    Html(render!(SERVE_ALL_STATUS, statuses => read))
}

/// Serves the status of a single server
async fn serve_single_status(server: String, status: Status) -> Html<String> {
    const SERVE_SINGLE_STATUS: &'static str = include_str!("../templates/single.html");

    let read = status.read().unwrap();
    let response = read.get(&server).unwrap();

    Html(render!(SERVE_SINGLE_STATUS, server => server, status => response))
}
