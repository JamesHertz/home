/*
 TODO:
    - Auth v1
        - [ ] store data on sqlite3
        - [ ] use argon2 for password generation and auth
        - [ ] have a in memory session manager
    - Auth v2
        - [ ] add timeouts between failed (do some more research)
            - per user?
            - per IP?
            - use cookies?
        - [ ] have 2FA
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
    Init
}



#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into())
        )
        .init();

    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Serve => home::run_server().await.unwrap(),
        _ => todo!()
    }

}
