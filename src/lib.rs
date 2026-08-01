pub mod server;
pub mod cli;

pub static HOME_VERSION : &str = env!("CARGO_PKG_VERSION");
pub use server::run_server;

