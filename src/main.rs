use std::{error::Error, net::IpAddr};

use clap::Parser;
use env_logger::Env;
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
    let env = Env::default().filter_or("RUST_LOG", "info");
    env_logger::init_from_env(env);
    match args.command {
        Commands::Server(ServerArgs {
            server_ip,
            port,
            tls_args,
        }) => {
            let server_ip: IpAddr = server_ip.parse()?;
            log::info!("starting in server mode with {server_ip}");
            let server_ins = server::Server::new(server_ip, port, tls_args).await?;
            server_ins.start().await?;
        }
        Commands::Client(ClientArgs {
            server_addr,
            local_net,
            request_port,
            tls_args,
        }) => {
            let server_addr = tokio::net::lookup_host(server_addr)
                .await
                .expect("unable to resolve address")
                .next()
                .expect("unable to resolve addr");
            log::info!(
                "starting in client mode with server addr:{server_addr} and exposing {local_net}"
            );
            let client_ins = client::Client::new(
                server_addr.ip(),
                server_addr.port(),
                local_net,
                request_port,
                tls_args,
            )
            .await?;
            client_ins.start().await?;
        }
    }
    Ok(())
}
