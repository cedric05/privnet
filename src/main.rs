use std::error::Error;

use clap::Parser;
use tokio;

mod client;
mod cmd;
mod message;
mod server;
mod utils;

use cmd::{ClientArgs, Commands, ServerArgs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = cmd::CmdArgs::parse();
    env_logger::init();
    match args.command {
        Commands::Server(ServerArgs { bind_addr }) => {
            log::info!("starting in server mode with {bind_addr}");
            let server_ins = server::Server::new(bind_addr).await?;
            server_ins.start().await?;
        }
        Commands::Client(ClientArgs {
            server_addr,
            local_net,
        }) => {
            log::info!(
                "starting in client mode with server addr:{server_addr} and exposing {local_net}"
            );
            let client_ins = client::Client::new(server_addr, local_net).await?;
            client_ins.start().await?;
        }
    }
    Ok(())
}
