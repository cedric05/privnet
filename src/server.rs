use std::error::Error;
use std::net::IpAddr;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use futures::{SinkExt, StreamExt};
use tokio_serde::formats::Json;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::message;
use crate::utils;

pub struct Server {
    pub tcp_listener: TcpListener,
    pub server_ip: IpAddr,
}

impl Server {
    pub async fn new(server_ip: IpAddr, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        log::debug!("establishing socket listener");
        let socket = TcpListener::bind((server_ip, port).clone()).await?;
        log::info!("server started started listening on {server_ip}:{port}");
        Ok(Self {
            server_ip,
            tcp_listener: socket,
        })
    }
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        loop {
            log::debug!("waiting for new socket connection");
            let (stream, new_conn_addr) = self.tcp_listener.accept().await?;
            log::info!("new client socket connection established to server {new_conn_addr}");
            let client_handler = ClientConnectionHandler::new(stream, self.server_ip).await?;
            tokio::spawn(async move {
                log::debug!("waiting for client connect for {new_conn_addr}");
                let _ = client_handler.listen().await;
                log::debug!("client disconnected {new_conn_addr}");
            });
        }
    }
}

pub struct ClientConnectionHandler {
    frame: tokio_serde::Framed<
        Framed<TcpStream, LengthDelimitedCodec>,
        message::ClientRequest,
        message::ServerResponse,
        Json<message::ClientRequest, message::ServerResponse>,
    >,
    server_ip: IpAddr,
    client_addr: std::net::SocketAddr,
}

impl ClientConnectionHandler {
    async fn new(
        stream: TcpStream,
        server_ip: IpAddr,
    ) -> Result<ClientConnectionHandler, Box<dyn std::error::Error>> {
        let client_addr = stream.peer_addr()?;
        let frame =
            tokio_util::codec::Framed::new(stream, tokio_util::codec::LengthDelimitedCodec::new());
        let frame: tokio_serde::Framed<
            Framed<TcpStream, LengthDelimitedCodec>,
            message::ClientRequest,
            message::ServerResponse,
            Json<message::ClientRequest, message::ServerResponse>,
        > = tokio_serde::Framed::new(frame, tokio_serde::formats::Json::default());
        return Ok(ClientConnectionHandler {
            frame,
            client_addr,
            server_ip,
        });
    }
    async fn listen(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (new_connection_sender, mut new_connection_reciever) = unbounded_channel::<()>();
        let mut socket_sender: Option<UnboundedSender<TcpStream>> = None;
        let mut already_connected = false;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(30))=>{
                    log::info!("no ping or no new connection recieved from client, closing connection");
                    return Ok(());
                }
                new_connection = new_connection_reciever.recv() => {
                    log::info!("new connection for listening port");
                    // TODO fix this match
                    let _new_connection = match new_connection{
                        Some(new_connection) => new_connection,
                        None => continue,
                    };
                    let client_connect_port = utils::new_port(None);
                    log::info!("new port in which client can connect is {}", client_connect_port);
                    self.frame.send(message::ServerResponse::NewConnection { client_connect_port }).await?;
                    log::debug!("sent request to client {}", client_connect_port);
                    log::debug!("listening on {}:{}", self.server_ip, client_connect_port);
                    let client_connect_socket = TcpListener::bind((self.server_ip, client_connect_port)).await?;
                    let (stream, remote_addr) = client_connect_socket.accept().await?;
                    if remote_addr != self.client_addr {
                        // TODO fix this
                        log::debug!("getting connections from unknown ip {}", remote_addr);
                    }
                    match &mut socket_sender{
                        Some(sender) => {
                            log::debug!("sent new socket connection to");
                            let _ = sender.send(stream)?;
                        },
                        None => {},
                    }
                }
                message =  self.frame.next()=>{
                    let message = match message {
                        Some(Ok(message)) => message,
                        Some(Err(message)) => {
                            log::info!("running into error {}", message);
                            continue;
                        },
                        // TODO look for client disconnect and close loop
                        None => continue,
                    };
                    match message {
                        message::ClientRequest::Ping { seq } => {
                            log::debug!("recieved ping from client");
                            // TODO act upon failure
                            let _ = self
                                .frame
                                .send(message::ServerResponse::Pong { seq: seq })
                                .await;
                        }
                        message::ClientRequest::ClientConnect(port) => {
                            log::info!("received cliet connect from client {}", self.client_addr);
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
                            let (_socket_sender, socket_reciever) = unbounded_channel::<TcpStream>();
                            socket_sender = Some(_socket_sender);
                            tokio::spawn({
                                log::info!("spawned new task for accepting connections from public");
                                let new_connection_sender = new_connection_sender.clone();
                                async move {
                                let _ = ClientConnectionHandler::loop_for_new_connections(
                                    port,
                                    self.server_ip,
                                    new_connection_sender,
                                    socket_reciever,
                                )
                                .await;
                            }});
                        }
                    }
                }
            }
        }
    }

    async fn loop_for_new_connections(
        port: u16,
        ip_addr: IpAddr,
        sender: UnboundedSender<()>,
        mut reciever: UnboundedReceiver<TcpStream>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("creating new server for listening");
        let socket = TcpListener::bind((ip_addr, port)).await?;
        log::info!("new tcp server listening on {}:{}", ip_addr, port);
        loop {
            log::debug!("accepting new connection on {}:{}", ip_addr, port);
            let (stream, remote_addr) = socket.accept().await?;
            log::info!(
                "accepted new connection on {}:{} from {}",
                ip_addr,
                port,
                remote_addr
            );
            sender.send(())?;
            log::debug!("sent signal to request new connection from client ");
            let stream_from_clinet = reciever.recv().await.unwrap();
            log::debug!("recieved new connection from client");
            tokio::spawn(async {
                log::debug!("proxying request");
                utils::proxy(stream, stream_from_clinet).await;
                log::debug!("proxying complete, connection closed");
            });
        }
    }
}
