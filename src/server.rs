#![allow(unused)]

use std::error::Error;
use axum::{
    Router, Json,
    routing::{get, post},
    http::{StatusCode, Request},
    response::{IntoResponse, Redirect, Html},
};

use axum_extra::extract::{
    CookieJar, cookie::{Cookie, SameSite}
};

use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer
};

use tokio::{
    net::TcpListener,
    fs
};

// auth imports
use argon2::Argon2;

use std::{
    sync::{Arc, RwLock},
    collections::HashMap,
    time::{Duration, SystemTime}
};

use serde::Deserialize;
use serde_json::json;

// use rand::{
//     TryRng,
//     rngs::SysRng
// };

pub struct ServerConfig {
    pub server_port : u16,
    pub server_address : String,
}

impl ServerConfig {
    fn version(&self) -> &'static str {
        crate::HOME_VERSION
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { 
            server_address : "localhost".to_string(),
            server_port: 8080,
        }
    }
}


struct App {
    pwd_hash_cfg : Argon2<'static>,
    // session_manager : SessionManager
}

impl App {
    fn new() -> Self {
        Self {
            pwd_hash_cfg: Argon2::default(),
            // session_manager : SessionManager::new(
            //     Duration::from_hours(24)
            // )
        }
    }
}

// struct SessionManager {
//     sessions : RwLock<HashMap<u32, Session>>,
//     // secure_randoms : Box<dyn CryptoRng>,
//     records_ttl : Duration,
// }
//
// #[derive(Clone)]
// struct Session { 
//     id : u32,
//     username : String,
//     expires_at : SystemTime
// }
//
// impl SessionManager {
//     fn new(records_ttl : Duration) -> Self {
//         Self {
//             sessions : RwLock::new( HashMap::new() ),
//             // secure_randoms : Box::new(secure_randoms),
//             records_ttl
//         }
//     }
//
//     // TODO: GIVE MORE THOUGHT TO THIS (->)
//     fn new_session(self, username : String) -> Session {
//         let mut rng = SysRng;
//         let session_id = rng.try_next_u32().unwrap();
//
//         let expires_at = SystemTime::now() + self.records_ttl;
//         let mut sessions = self.sessions.write().unwrap();
//
//         let session = Session {
//             id: session_id,
//             username,
//             expires_at,
//         };
//
//         assert!(
//             sessions.insert(session_id, session.clone()).is_none(),
//             "BAD NEWS: either a BUG or the world is ending :("
//         );
//
//         session
//     }
//
//     fn validate_session_id(self, session_id : u32) -> Option<String> {
//         let sessions = self.sessions.read().unwrap();
//
//         sessions.get(&session_id)
//             .map(|s| s.username.clone())
//     }
//
// }
//

#[derive(Deserialize, Debug)]
struct AuthCredentials {
    login    : String,
    password : String
}

async fn login(jar : CookieJar, Json(creds) : Json<AuthCredentials>) -> impl IntoResponse {
    if let Some(cookie) = jar.get("session_id") && cookie.value() == "home_confirmed" {
        Redirect::to("/home").into_response()
    } else if creds.login == "jhertz" && creds.password == "1234" {
        let cookie = Cookie::build(("session_id", "home_confirmed"))
            .max_age(time::Duration::minutes(1))
            .same_site(SameSite::Lax)
            .path("/")
            .http_only(true)
            // .secure()
            .build();
        (
            jar.add(cookie),
            Json(json!(
                { "redirectUrl": "/home" }
            ))
        ).into_response()
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

async fn home(jar : CookieJar) -> Result<Html<String>, Redirect> {
    if let Some(cookie) = jar.get("session_id") && cookie.value() == "home_confirmed" {
        Ok(Html(
            "<div>Welcome <strong>username</strong></div>".to_string()
        ))
    } else {
        Err(Redirect::temporary("/home/login"))
    }
}

async fn login_page(jar : CookieJar) -> impl IntoResponse {
    if let Some(cookie) = jar.get("session_id") && cookie.value() == "home_confirmed" {
        Redirect::to("/home").into_response()
    } else {
        match fs::read_to_string("./static/home.html").await {
            Ok(content) => Html(content).into_response(),
            Err(_) => StatusCode::NOT_FOUND.into_response()
        }
    }
}

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    run_server_with_cfg(ServerConfig::default()).await
}

pub async fn run_server_with_cfg(cfg : ServerConfig) -> Result<(), Box<dyn Error>> {
    let base_addr = format!("{}:{}", cfg.server_address, cfg.server_port);
    let listener = TcpListener::bind(&base_addr).await?;

    let state = Arc::new(App::new());
    let api_endpoints = Router::new()
        .route_service("/version", get(async || {
            Json( json!({ "version": crate::HOME_VERSION }) )
        }))
        .route_service("/login", post(login));

    // TODO: send this to a config file
    let router = Router::new()
        // TODO: fix this
        // .route_service("/home/login", ServeFile::new("static/home.html"))
        .route_service("/home/login", get(login_page))
        .route_service("/home", get(home))
        // .route_service("", post(login))
        .fallback_service(ServeDir::new("static/"))
        .nest("/api/", api_endpoints)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!("Serving Home @ v{} at http://{}", cfg.version(), base_addr);
    axum::serve(listener, router).await?;

    Ok(())
}

/*
TODO:
    - connect to Sqlite
    - get AUTH working
    - add some cli commmands to home
*/
