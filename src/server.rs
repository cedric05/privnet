use std::error::Error;
use std::net::IpAddr;
use std::pin::Pin;
use std::time::Duration;

use openssl::ssl::{Ssl, SslAcceptor};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use futures::{SinkExt, StreamExt};
use tokio_openssl::SslStream;
use tokio_serde::formats::Json;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::cmd::TlsArgs;
use crate::message;
use crate::utils::{self, load_acceptor, proxy};

pub struct Server {
    pub socket: TcpListener,
    pub server_ip: IpAddr,
    pub acceptor: SslAcceptor,
}

impl Server {
    pub async fn new(
        server_ip: IpAddr,
        port: u16,
        tls_args: TlsArgs,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        log::debug!("establishing socket listener");
        let acceptor = load_acceptor(tls_args).unwrap();
        let socket = TcpListener::bind((server_ip, port).clone()).await.expect(
            "unable to assign server requested addr, please check port is free or ip is assignable",
        );
        log::info!("server started started listening on {server_ip}:{port}");
        Ok(Self {
            server_ip,
            socket,
            acceptor,
        })
    }
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        loop {
            log::debug!("waiting for new socket connection");
            let (stream, new_conn_addr) = match self.socket.accept().await {
                Ok((stream, new_conn_addr)) => (stream, new_conn_addr),
                Err(error) => {
                    log::error!("socket accept failed with error {}", error);
                    continue;
                }
            };
            log::info!("new client socket connection established to server {new_conn_addr}");
            tokio::spawn({
                let server_ip = self.server_ip.clone();
                let acceptor = self.acceptor.clone();
                async move {
                    let ssl = Ssl::new(acceptor.context()).unwrap();
                    let mut stream = SslStream::new(ssl, stream).unwrap();
                    Pin::new(&mut stream).accept().await.unwrap();
                    let client_handler = ClientConnectionHandler::new(stream, server_ip);
                    log::debug!("waiting for client connect for {new_conn_addr}");
                    let _ = client_handler.listen().await;
                    log::debug!("client disconnected {new_conn_addr}");
                }
            });
        }
    }
}

pub struct ClientConnectionHandler {
    frame: tokio_serde::Framed<
        Framed<SslStream<TcpStream>, LengthDelimitedCodec>,
        message::ClientRequest,
        message::ServerResponse,
        Json<message::ClientRequest, message::ServerResponse>,
    >,
    server_ip: IpAddr,
    client_addr: Option<std::net::SocketAddr>,
}

impl ClientConnectionHandler {
    fn new(stream: SslStream<TcpStream>, server_ip: IpAddr) -> ClientConnectionHandler {
        // TODO fix client_addr
        let client_addr = None; // stream.peer_addr().ok();
        let frame =
            tokio_util::codec::Framed::new(stream, tokio_util::codec::LengthDelimitedCodec::new());
        let frame: tokio_serde::Framed<
            Framed<SslStream<TcpStream>, LengthDelimitedCodec>,
            message::ClientRequest,
            message::ServerResponse,
            Json<message::ClientRequest, message::ServerResponse>,
        > = tokio_serde::Framed::new(frame, tokio_serde::formats::Json::default());
        return ClientConnectionHandler {
            frame,
            client_addr,
            server_ip,
        };
    }
    async fn listen(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (new_connection_sender, mut new_connection_reciever) = unbounded_channel::<TcpStream>();
        let mut already_connected = false;
        let mut fut: Option<tokio::task::JoinHandle<_>> = None;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(30))=>{
                    log::info!("no ping or no new connection recieved from client, closing connection");
                    return Ok(());
                }
                new_connection = new_connection_reciever.recv() => {
                    log::info!("new connection for listening port");
                    // TODO fix this match
                    let new_connection = match new_connection{
                        Some(new_connection) => new_connection,
                        None => continue,
                    };
                    let stream = match get_new_stream_from_client(self.server_ip, &mut self.frame, self.client_addr.map(|x|x.ip())).await{
                        Ok(stream)=> stream,
                        Err(err) => {
                            log::error!("unable to get connection to local net, {err}");
                            continue;
                        }
                    };
                    fut = Some(tokio::spawn(async {
                        proxy(new_connection, stream).await;
                    }));
                }
                message =  self.frame.next()=>{
                    let message = match message {
                        Some(Ok(message)) => message,
                        Some(Err(message)) => {
                            log::info!("running into error {}", message);
                            continue;
                        },
                        // TODO look for client disconnect and close loop
                        None => {
                            fut.map(|x|x.abort());
                            return Ok(())
                        },
                    };
                    match message {
                        message::ClientRequest::Ping { seq } => {
                            log::debug!("recieved ping from client");
                            // TODO act upon failure
                                self
                                .frame
                                .send(message::ServerResponse::Pong { seq: seq })
                                .await.unwrap_or(());
                        }
                        message::ClientRequest::ClientConnect(port) => {
                            log::info!("received cliet connect from client {:?}", self.client_addr);
                            if already_connected {
                                // for single client, there is only one client connect
                                // if already connected, don't act
                                log::info!("client already established ClientConnect, ignoring this message");
                                continue;
                            }
                            already_connected = true;
                            // availabile port
                            let port = utils::new_port(port);
                            log::info!("created new exposed port on server `{}:{port}`", self.server_ip);
                            println!("created new exposed port on server `{}:{port}`", self.server_ip);
                            // TODO check response
                            let _ = self
                                .frame
                                .send(message::ServerResponse::Ok {
                                    end_user_port: port,
                                })
                                .await;
                            fut = Some(tokio::spawn({
                                log::info!("spawned new task for accepting connections from public");
                                let new_connection_sender = new_connection_sender.clone();
                                async move {
                                let _ = ClientConnectionHandler::loop_for_new_connections(
                                    port,
                                    self.server_ip,
                                    new_connection_sender,
                                )
                                .await;
                            }}));
                        }
                    }
                }
            }
        }
    }

    async fn loop_for_new_connections(
        port: u16,
        ip_addr: IpAddr,
        sender: UnboundedSender<TcpStream>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("creating new server for listening");
        // here we already verified port is available.
        let socket = TcpListener::bind((ip_addr, port)).await.unwrap();
        log::info!("new tcp server listening on {}:{}", ip_addr, port);
        loop {
            log::debug!("accepting new connection on {}:{}", ip_addr, port);
            let (stream, remote_addr) = match socket.accept().await {
                Ok((stream, remote_addr)) => (stream, remote_addr),
                Err(error) => {
                    log::error!("unable to accept new connection, {error}");
                    continue;
                }
            };
            log::info!(
                "accepted new connection on {}:{} from {}",
                ip_addr,
                port,
                remote_addr
            );
            if let Err(error) = sender.send(stream) {
                log::warn!("unable to send notification of new connection to client");
                log::error!("sending message failed with error {}", error);
                continue;
            }
        }
    }
}

async fn get_new_stream_from_client(
    server_ip: IpAddr,
    frame: &mut tokio_serde::Framed<
        Framed<SslStream<TcpStream>, LengthDelimitedCodec>,
        message::ClientRequest,
        message::ServerResponse,
        Json<message::ClientRequest, message::ServerResponse>,
    >,
    client_addr: Option<IpAddr>,
) -> Result<TcpStream, Box<dyn std::error::Error>> {
    let client_connect_socket = TcpListener::bind((server_ip, 0)).await?;
    let client_connect_port = client_connect_socket.local_addr().unwrap().port();
    log::info!(
        "new port in which client can connect is {}",
        client_connect_port
    );
    frame
        .send(message::ServerResponse::NewConnection {
            client_connect_port,
        })
        .await?;
    log::debug!("sent request to client {}", client_connect_port);
    log::debug!("listening on {}:{}", server_ip, client_connect_port);
    let (stream, remote_addr) = client_connect_socket.accept().await?;
    if Some(remote_addr.ip()) != client_addr {
        // TODO fix this
        log::debug!("getting connections from unknown ip {}", remote_addr);
    }
    Ok(stream)
}
