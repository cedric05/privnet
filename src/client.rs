use std::error::Error;
use std::net::IpAddr;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_serde::formats::Json;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::message;
use crate::utils;

pub struct Client {
    pub frame: tokio_serde::Framed<
        Framed<TcpStream, LengthDelimitedCodec>,
        message::ServerResponse,
        message::ClientRequest,
        Json<message::ServerResponse, message::ClientRequest>,
    >,
    pub server_ip: IpAddr,
    pub local_addr: String,
    pub request_port: Option<u16>,
}

impl Client {
    pub async fn new(
        server_ip: IpAddr,
        port: u16,
        local_addr: String,
        request_port: Option<u16>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = TcpStream::connect((server_ip, port)).await?;
        log::debug!("successfully connected to server");
        let server_resolv_addr = stream.local_addr()?;
        log::debug!("server resolved addr: {server_resolv_addr}");
        let frame =
            tokio_util::codec::Framed::new(stream, tokio_util::codec::LengthDelimitedCodec::new());
        let frame = tokio_serde::Framed::new(frame, tokio_serde::formats::Json::default());
        Ok(Self {
            frame,
            server_ip,
            local_addr,
            request_port,
        })
    }
    pub async fn start(mut self) -> Result<(), Box<dyn Error>> {
        log::debug!("sending client connect to server");
        self.frame
            .send(message::ClientRequest::ClientConnect(self.request_port))
            .await?;
        let mut seq: u32 = 1;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(10))=>{
                    log::debug!("sending sequence to server sequence:{}", seq);
                    let _ = self.frame.send(message::ClientRequest::Ping { seq: seq }).await;
                    seq += 1;
                }
                message = self.frame.next() =>{
                    let message = match message {
                        Some(Ok(message)) => message,
                        // look for errors and gracefully exit, so that it can retry connecting to serverrecieved ping from client
                        _ => continue,
                    };
                    match message {
                        message::ServerResponse::Ok { end_user_port } => {
                            log::info!(
                                "client connect successful. server responded with port {}",
                                end_user_port
                            );
                            println!(
                                "connect to server address {}:{}",
                                self.server_ip,
                                end_user_port
                            );
                        }
                        message::ServerResponse::Pong { seq: _seq } => {
                            log::debug!("server responded with pong! sequence: {}", _seq);
                        }
                        message::ServerResponse::NewConnection {
                            client_connect_port,
                        } => {
                            log::info!(
                                "server requested new connection to port {}",
                                client_connect_port
                            );

                            // tokio::time::sleep(Duration::from_secs(10)).await;

                            // TODO, unable to connect to server should not crash client
                            let server_stream =
                                TcpStream::connect((self.server_ip, client_connect_port)).await?;

                            let local_net_stream = TcpStream::connect(self.local_addr.clone()).await?;
                            tokio::spawn(async {
                                utils::proxy(local_net_stream, server_stream).await;
                            });
                        }
                    }
                }
            }
        }
    }
}
