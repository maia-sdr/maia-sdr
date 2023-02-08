//! maia-httpd CLI arguments.
//!
//! This module contains the definition of the CLI arguments for the maia-httpd
//! application.

use clap::Parser;
use std::net::SocketAddr;

/// maia-httpd CLI arguments.
#[derive(Parser, Debug, Clone, Eq, PartialEq, Hash)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Listen address for the HTTP server
    #[clap(long, default_value = "0.0.0.0:8000")]
    pub listen: SocketAddr,
}
