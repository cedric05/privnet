use std::error::Error;
use std::io;
use std::net::IpAddr;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rustls::pki_types;
use tokio::net::TcpStream as TokioTcpStream;
use tokio::task::JoinHandle;
use tokio_rustls::TlsConnector;
use tokio_serde::formats::Json;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::cmd::TlsArgs;
use crate::message;
use crate::utils;
use crate::utils::get_client_config;

type TcpStream = tokio_rustls::client::TlsStream<TokioTcpStream>;

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
        tls_args: TlsArgs,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = get_client_config(&tls_args);
        let connector = TlsConnector::from(config);
        let stream = TokioTcpStream::connect((server_ip, port))
            .await
            .expect("Unable to connect to server");

        let domain = pki_types::ServerName::try_from("localhost")
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?
            .to_owned();

        let stream = connector.connect(domain, stream).await?;
        log::debug!("successfully connected to server");
        // let server_resolv_addr = stream.local_addr()?;
        // log::debug!("server resolved addr: {server_resolv_addr}");
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
            .await
            .expect("unable to send client connect protocol with server");
        let mut seq: u32 = 1;
        let mut fut: Option<JoinHandle<_>> = None;
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
                        Some(Err(message))=> {
                            log::debug!("ran into error {}", message);
                            continue;
                        }
                        // look for errors and gracefully exit, so that it can retry connecting to serverrecieved ping from client
                        None => {
                            log::warn!("server disconnected aborting client" );
                            fut.map(|x| x.abort());
                            return Ok(());
                        },
                    };
                    match message {
                        message::ServerResponse::Ok { end_user_port } => {
                            log::info!(
                                "client connect successful. server responded with port {}",
                                end_user_port
                            );
                            let requested_satisfied = match self.request_port.as_ref(){
                                Some(request_port)=> request_port == &end_user_port,
                                None=>true
                            };
                            if requested_satisfied {
                                println!(
                                    "connect to server address {}:{}",
                                    self.server_ip,
                                    end_user_port
                                );
                            } else {
                                println!(
                                    "server coudn't allocate requested port {:?} connect to server address {}:{}",
                                    self.request_port,
                                    self.server_ip,
                                    end_user_port
                                );
                            }
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
                            // wait for 1 second for (client to setup)
                            tokio::time::sleep(Duration::from_millis(60)).await;
                            _ = connect_server_n_local((self.server_ip, client_connect_port).into(), &self.local_addr).await.map(|(s1, s2)|{
                                fut = Some(tokio::spawn(async {
                                    utils::proxy(s1, s2).await;
                                }));
                            });

                        }
                    }
                }
            }
        }
    }
}

async fn connect_server_n_local(
    server_addr: std::net::SocketAddr,
    local_addr: &str,
) -> Result<(TokioTcpStream, TokioTcpStream), Box<dyn std::error::Error>> {
    let server_stream = TokioTcpStream::connect(server_addr).await?;
    let local_net_stream = TokioTcpStream::connect(local_addr).await.map_err(|x| {
        log::error!("looks to be local_net stream is not connectable");
        x
    })?;
    Ok((server_stream, local_net_stream))
}
