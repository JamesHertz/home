use std::sync::Arc;
use time::Duration;

use axum::{
    Json, Router, extract::State, http::StatusCode, response::{IntoResponse, Redirect}, routing::post
};

use axum_extra::extract::cookie::{
    Cookie, CookieJar, SameSite
};

use tokio::{ fs, net::TcpListener };
use tower_http::services::{ServeDir, ServeFile};


#[derive(serde::Deserialize)]
struct Config {
    master_key     : Login,
    special_cookie : String
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
struct Login {
    username : String,
    password : String
}

async fn login(
    mut jar : CookieJar,
    State(cfg) : State<Arc<Config>>, 
    Json(credentials): Json<Login>,
) -> impl IntoResponse {

    if 
        let Some(cookie) = jar.get("session_id") 
            && cookie.value() == cfg.special_cookie
    {
        return Redirect::to("/home/sharing").into_response();
    }

    if credentials == cfg.master_key {

        // FIXME: get rid of this stupidity and do cookies parsing yourself :)
        jar = jar.add(
            Cookie::build(("session_id", cfg.special_cookie.clone()))
                .http_only(true)
                .same_site(SameSite::Lax)
                .max_age(Duration::minutes(2))
                .build()
        );

        (jar, Redirect::to("/home/sharing")).into_response()
     } else {
        println!("/home/login - auth failed");
        (StatusCode::UNAUTHORIZED, "Invalid username or password").into_response()
    }
}


// async fn sharing(jar : CookieJar, State(cfg) : State<Config>) -> impl IntoResponse {
//     let cookie = match jar.get("session_id") {
//         Some(cookie) => cookie,
//         None => { return StatusCode::UNAUTHORIZED.into_response(); }
//     }
// }


#[tokio::main]
async fn main() {

    let cfg : Arc<Config> = Arc::new({
        let data = 
            fs::read_to_string( ".config.json")
            .await
            .expect("failed to read `config file`");

        serde_json::from_str(&data)
            .expect("failed to parse `config file`")
    });

    let router = Router::new()
        .route("/api/login", post(login))
        .route_service("/home", ServeFile::new("static/home.html"))
        .fallback_service(ServeDir::new("static/"))
        .with_state(cfg);

    // /post/home {"user": "user", "password": "password" }
    // setup a cookie and ther esponse will be aredirect
    let listener = TcpListener::bind("localhost:8080").await.unwrap();
    println!("serving website at http://localhost:8080/");
    let _ = axum::serve(listener, router).await;
}
