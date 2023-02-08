use anyhow::Result;
use clap::Parser;
use maia_httpd::{app::App, args::Args};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    App::new(&Args::parse()).await?.run().await
}
