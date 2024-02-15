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
    #[clap(default_value = "0.0.0.0")]
    pub(crate) server_ip: String,

    #[clap(default_value_t = 1420)]
    pub(crate) port: u16,
}

#[derive(Args, Debug)]
pub(crate) struct ClientArgs {
    /// client connects to this address.
    #[clap(default_value = "0.0.0.0:1420")]
    pub(crate) server_addr: String,
    /// local server you want to expose via server, if not set, exposes postgres
    #[clap(default_value = "localhost:5432")]
    pub(crate) local_net: String,

    /// Request specific port with server
    pub(crate) request_port: Option<u16>,
}
