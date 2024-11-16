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
    /// Listen address for the HTTPS server
    #[clap(long, default_value = "0.0.0.0:443")]
    pub listen_https: SocketAddr,
}

#[cfg(feature = "uclibc")]
impl Default for Args {
    fn default() -> Args {
        Args {
            listen: "0.0.0.0:8000".parse().unwrap(),
            listen_https: "0.0.0.0:443".parse().unwrap(),
        }
    }
}
