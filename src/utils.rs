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

pub fn new_port(check_port: Option<u16>) -> u16 {
    // TODO fix unwrap
    let port = check_port.unwrap_or_default();
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(server) => server.local_addr().unwrap().port(),
        Err(_) => TcpListener::bind(("0.0.0.0", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port(),
    }
}
