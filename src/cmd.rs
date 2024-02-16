use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct CmdArgs {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Starts in server mode(servers behind NAT are exposed via this)
    Server(ServerArgs),
    /// Starts in client mode(client runs in same network as NAT and connects to server to expose local servers)
    Client(ClientArgs),
}

#[derive(Args, Debug)]
pub(crate) struct ServerArgs {
    // server bind address
    #[clap(short, long, default_value = "0.0.0.0")]
    pub(crate) server_ip: String,

    #[clap(short, long, default_value_t = 1420)]
    pub(crate) port: u16,

    #[command(flatten)]
    pub(crate) tls_args: TlsArgs,
}

#[derive(Args, Debug)]
pub(crate) struct ClientArgs {
    /// local server you want to expose via server, if not set, exposes postgres
    #[clap(short, long, default_value = "localhost:5432")]
    pub(crate) local_net: String,

    /// client connects to this address.
    #[clap(short, long, default_value = "0.0.0.0:1420")]
    pub(crate) server_addr: String,

    /// Request specific port with server
    #[clap(short, long)]
    pub(crate) request_port: Option<u16>,

    #[command(flatten)]
    pub(crate) tls_args: TlsArgs,
}

#[derive(Args, Debug)]
pub(crate) struct TlsArgs {
    #[clap(long, requires = "key")]
    pub(crate) cert: Option<PathBuf>,

    #[clap(long, requires = "ca")]
    pub(crate) key: Option<PathBuf>,

    #[clap(long, requires = "cert")]
    pub(crate) ca: Option<PathBuf>,
}
