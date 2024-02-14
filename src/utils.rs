use std::net::TcpListener;

use tokio::io::{self, AsyncRead, AsyncWrite};

pub async fn proxy<T1, T2>(s1: T1, s2: T2)
where
    T1: AsyncRead + AsyncWrite + Unpin,
    T2: AsyncRead + AsyncWrite + Unpin,
{
    let (mut read_1, mut write_1) = io::split(s1);
    let (mut read_2, mut write_2) = io::split(s2);
    tokio::select! {
        _=io::copy(&mut read_1, &mut write_2)=>{},
        _=io::copy(&mut read_2, &mut write_1)=>{}
    }
    println!("closing connection");
}

pub fn new_port() -> u16 {
    // TODO fix unwrap
    let server = TcpListener::bind(("0.0.0.0", 0)).unwrap();
    // TODO fix unwrap
    let addr = server.local_addr().unwrap();
    println!("port is {}", addr);
    addr.port()
}
