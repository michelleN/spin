use anyhow::Result;
use log as rust_log;
use spin_sdk::{
    http::{Request, Response},
    http_component,
};

use spin_sdk::wit::wasi::logging::logging::{log, Level};

/// A simple Spin HTTP component.
#[http_component]
fn hello_world(req: Request) -> Result<Response> {
    log(Level::Info, "hello", "goodbye");
    rust_log::info!("{:?}", req.headers());
    Ok(http::Response::builder()
        .status(200)
        .header("foo", "bar")
        .body(Some("Hello, Fermyon!\n".into()))?)
}
