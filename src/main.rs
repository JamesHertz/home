/*
 TODO:
    - Auth v1
        - [x] store data on sqlite3
        - [x] use argon2 for password generation and auth
        - [x] have a in memory session manager
    - Lading page
        - [ ] make a good looking landing page
        - [ ] implement a TODO app
        - [ ] implement a file uploading app
    - Properly organize your code
        - [ ] do proper modules
        - [ ] have a proper config file (.ini or .Yaml or ENV)
        - [ ] Containrize your app and ship it
    - Auth v2
        - [ ] Look into axum custom auth solution
        - [ ] enforce strong passwords
        - [ ] add timeouts between failed (do some more research)
            - per user?
            - per IP?
            - use cookies?
        - [ ] have 2FA?
*/

use tracing_subscriber::EnvFilter;
use clap::{
    Parser, Subcommand
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command : Option<Commands>
}

#[derive(Subcommand)]
enum Commands {
    /// Runs home server
    Serve,

    /// Creates and initializes the database
    InitApp { admin_pwd : String, db_file : String }
}

#[tokio::main]
async fn main() -> home::server::Result<()> {

    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into())
        )
        .init();

    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Serve => home::run_server().await?,
        Commands::InitApp { 
            admin_pwd, db_file 
        } => home::cli::init_app(&admin_pwd, &db_file).await?
    }

    Ok(())
}
