pub mod server;

pub static HOME_VERSION : &str = env!("CARGO_PKG_VERSION");
pub use server::run_server;

