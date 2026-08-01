use std::{
    collections::HashMap, error::Error, sync::{Arc, RwLock}, time::{Duration, SystemTime}
};

use axum::{
    Json, Router, 
    http::StatusCode, 
    response::{Html, IntoResponse, Redirect, Response}, 
    routing::{get, post},
    extract::State
};

use axum_extra::extract::{
    CookieJar, cookie::{Cookie, SameSite}
};

use tower_http::{
    services::ServeDir,
    trace::TraceLayer
};

use tokio::{
    net::TcpListener, fs
};

use serde::{Deserialize, Serialize};
use serde_json::json;

// auth imports
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::rand_core::{OsRng, RngCore}
};

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

#[derive(Debug)]
pub struct ServerConfig {
    pub server_port    : u16,
    pub server_address : String,
    pub database_url   : String,
    pub cookie_ttl_ms  : u64,
    pub session_cleanup_interval_ms : u64
        
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
            database_url: "sqlite:data.db".to_string(),
            cookie_ttl_ms: Duration::from_mins(1).as_millis() as u64,
            session_cleanup_interval_ms: Duration::from_hours(1).as_millis() as u64
        }
    }
}


struct App {
    argon_cfg : Argon2<'static>,
    db : SqlitePool,
    session_manager : SessionManager
}

impl App {
    async fn build(cfg : &ServerConfig) -> Result<Self> {
        Ok(Self {
            argon_cfg: Argon2::default(),
            db: SqlitePoolOptions::new()
                    .max_connections(1)
                    .connect(&cfg.database_url)
                    .await?,
            session_manager : SessionManager::new(
                Duration::from_millis(cfg.cookie_ttl_ms)
            )
       })
    }

    fn get_argon2_cfg(&self) -> Argon2<'_> {
        self.argon_cfg.clone()
    }
}

pub type StdResult<T, E> = std::result::Result<T, E>;
pub type Result<T> = std::result::Result<T, AppError>;

// #[derive(Debug)]
// struct StrError(String);
//
// impl std::fmt::Display for StrError {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         write!(f, "StrError: {}", self.0)
//     }
// }
//
// impl std::error::Error for StrError {}

#[derive(Debug)]
pub enum AppError {
    WithResponse(Box<Response>),
    InteralError(Box<dyn Error>),
}

impl AppError {
    pub fn from_http_response(value : impl IntoResponse) -> Self {
        AppError::WithResponse(Box::new(value.into_response()))
    }
        
    pub fn fail_with<T>(value : impl IntoResponse) -> StdResult<T, Self> {
        Err( Self::from_http_response(value) )
    }
}

impl <T : Error + 'static> From<T> for AppError {
    fn from(value : T) -> Self {
        AppError::InteralError(Box::new(value))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::WithResponse(r) => *r,
            AppError::InteralError(error) => {
                tracing::error!("internal server error: {error}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

struct SessionManager {
    sessions : RwLock<HashMap<u32, Session>>,
    // secure_randoms : Box<dyn CryptoRng>,
    records_ttl : Duration,
}

#[derive(Clone)]
struct Session { 
    id : u32,
    user_login : String,
    expires_at : SystemTime
}

impl SessionManager {
    fn new(records_ttl : Duration) -> Self {
        Self {
            sessions : RwLock::new( HashMap::new() ),
            // secure_randoms : Box::new(secure_randoms),
            records_ttl
        }
    }

    // TODO: GIVE MORE THOUGHT TO THIS (->)
    fn new_session(&self, user_login : &str) -> Session {
        let mut rng = OsRng;
        let session_id = rng.next_u32();

        let expires_at = SystemTime::now() + self.records_ttl;
        let mut sessions = self.sessions.write().unwrap();

        let session = Session {
            id: session_id,
            user_login: user_login.to_string(),
            expires_at,
        };

        assert!(
            sessions.insert(session_id, session.clone()).is_none(),
            "BAD NEWS: either a BUG or the world is ending :("
        );

        session
    }

    fn validate_session_id(&self, session_id : u32) -> Option<Session> {
        let current_time = SystemTime::now();
        let sessions = self.sessions.read().unwrap();

        if let Some(session) = sessions.get(&session_id) 
            && current_time < session.expires_at
        {
            Some(session.clone())
        } else {
            None
        }
    }

    // TODO: think of a strategy for this
    fn delete_expired(&self) {
        let mut sessions = self.sessions.write().unwrap();
        let current_time = SystemTime::now();
        let expiried_sessions : Vec<_>= sessions.values().filter_map(
            |s| {
                if s.expires_at <= current_time {
                    Some(s.id)
                } else {
                    None
                }
            }
        ).collect();

        for session_id in &expiried_sessions {
            assert!( sessions.remove(session_id).is_some() )
        }

        tracing::info!(
            "Session cleaning job completed!! Delete {} expired sessions. There are {} active sessions.", 
            &expiried_sessions.len(), &sessions.len()
        );
    }

}

#[derive(Deserialize, Debug)]
struct AuthCredentials {
    login    : String,
    password : String
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthResponse {
    redirect_url : String
}

async fn login(
    State(app) : State<Arc<App>>,
    jar : CookieJar,
    Json(creds) : Json<AuthCredentials>
) -> Result<Response> {

    // TODO: does it really make sense to return the redirect link?
    if is_request_authenticated(&app, &jar).is_some() {
        return Ok(
            Json(AuthResponse {
                redirect_url : "/home".to_string()
            }).into_response()
        );
    }

    if creds.login.is_empty() || creds.password.is_empty() {
        return AppError::fail_with(StatusCode::BAD_REQUEST)
    }

    let pwd_hash = 
        sqlx::query_scalar!("SELECT pwd_hash FROM Users where login = ?", creds.login)
            .fetch_optional(&app.db)
            .await?;
        
    if let Some(pwd_hash) = pwd_hash {
        let argon_cfg = app.get_argon2_cfg();
        let parsed_hash = PasswordHash::new(&pwd_hash)?;

        if argon_cfg.verify_password(creds.password.as_bytes(), &parsed_hash).is_ok() {
            let session =  app.session_manager.new_session(&creds.login);

            let cookie = Cookie::build(("session_id", session.id.to_string()))
                    .max_age(time::Duration::minutes(1))
                    .same_site(SameSite::Lax)
                    .path("/")
                    .http_only(true)
                    // .secure()
                    .build();


            assert!(
                app.session_manager.validate_session_id(session.id).is_some()
            );

            return Ok(
                (
                    jar.add(cookie),
                    Json(AuthResponse { redirect_url : "/home".to_string() })
                ).into_response()
            );
        }
    }

    AppError::fail_with(StatusCode::UNAUTHORIZED)
}

async fn home(app : State<Arc<App>>, jar : CookieJar) -> Result<Html<String>> {
    if let Some(session) = is_request_authenticated(&app.0, &jar) {
        let username = 
            sqlx::query_scalar!("SELECT username FROM Users WHERE login = ?", session.user_login)
                .fetch_optional(&app.db)
                .await?;
            
        Ok(Html(
            format!("<div>Welcome <strong>{}</strong></div>", username.unwrap())
        ))
    } else {
        AppError::fail_with(
            Redirect::temporary("/home/login")
        )
    }
}

fn is_request_authenticated(app : &App, jar : &CookieJar) -> Option<Session> {
    let cookie = jar.get("session_id")?;
    let Ok(session_id) = cookie.value().parse::<u32>() else { return None };
    app.session_manager.validate_session_id(session_id)
}

async fn login_page(app : State<Arc<App>>, jar : CookieJar) -> Result<Html<String>> {
    if is_request_authenticated(&app.0, &jar).is_some() {
        AppError::fail_with( Redirect::to("/home") )
    } else {
        match fs::read_to_string("static/home.html").await {
            Ok(content) => Ok(Html(content)),
            Err(_) => AppError::fail_with(
                StatusCode::NOT_FOUND
            )
        }
    }
}

pub async fn run_server() -> Result<()> {
    run_server_with_cfg(ServerConfig::default()).await
}

pub async fn run_server_with_cfg(cfg : ServerConfig) -> Result<()> {
    let base_addr = format!("{}:{}", cfg.server_address, cfg.server_port);
    let listener = TcpListener::bind(&base_addr).await?;
    let state = Arc::new(
        App::build(&cfg).await?
    );

    {
        let state = state.clone();
        let interval_duration = Duration::from_millis(
            cfg.session_cleanup_interval_ms
        );
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                interval_duration
            );

            loop {
                interval.tick().await;
                state.session_manager.delete_expired();
            }
        });
    }

    let api_endpoints = Router::new()
        .route("/version", get(async || {
            Json( json!({ "version": crate::HOME_VERSION }) )
        }))
        .route("/login", post(login));

    // let protected = Router::new()
    //  .layer(
    //     middleware::from_fn_with_state(
    //         state.clone(),
    //         auth_middleware,
    //     )
    // );

    // TODO: send this to a config file
    let router = Router::new()
        .route("/home/login", get(login_page))
        .route("/home", get(home))
        .nest("/api/", api_endpoints)
        .fallback_service(ServeDir::new("static/"))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!("Serving Home @ v{} at http://{}", cfg.version(), base_addr);
    axum::serve(listener, router).await?;

    Ok(())
}
