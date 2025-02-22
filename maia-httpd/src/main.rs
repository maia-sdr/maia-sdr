use anyhow::Result;
#[cfg(not(feature = "uclibc"))]
use clap::Parser;
use maia_httpd::{app::App, args::Args};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    // workaround for https://github.com/rust-lang/rust/issues/112488
    #[cfg(feature = "uclibc")]
    let args = Args::default();
    #[cfg(not(feature = "uclibc"))]
    let args = Args::parse();

    App::new(&args).await?.run().await
}
