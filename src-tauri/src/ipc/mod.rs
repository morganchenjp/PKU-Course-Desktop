//! Custom URI-scheme protocol (`pku-ipc://`) used by inject scripts to
//! talk to the Rust backend without going through `window.__TAURI__`
//! (which is unavailable on remote URLs due to CSP / capability rules).

pub mod bridge;
pub mod routes;

pub use routes::handle;
