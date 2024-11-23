//! maia-httpd CLI arguments.
//!
//! This module contains the definition of the CLI arguments for the maia-httpd
//! application.

use clap::Parser;
use std::{net::SocketAddr, path::PathBuf};

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
    /// Path to SSL certificate for HTTPS server
    ///
    /// Unless both the SSL certificate and key are specified, the HTTPS server
    /// is disabled.
    #[clap(long)]
    pub ssl_cert: Option<PathBuf>,
    /// Path to SSL key for HTTPS server
    ///
    /// Unless both the SSL certificate and key are specified, the HTTPS server
    /// is disabled.
    #[clap(long)]
    pub ssl_key: Option<PathBuf>,
    /// Path to CA certificate for HTTPS server
    ///
    /// The CA certificate is accessible on /ca.crt of the web API if this
    /// option is provided.
    #[clap(long)]
    pub ca_cert: Option<PathBuf>,
}

#[cfg(feature = "uclibc")]
impl Default for Args {
    fn default() -> Args {
        Args {
            listen: "0.0.0.0:8000".parse().unwrap(),
            listen_https: "0.0.0.0:443".parse().unwrap(),
            ssl_cert: None,
            ssl_key: None,
            ca_cert: None,
        }
    }
}
