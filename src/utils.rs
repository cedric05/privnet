use std::{error::Error, fs, net::TcpListener};

use openssl::{
    ssl::{
        SslAcceptor, SslConnector, SslFiletype, SslMethod, SslSessionCacheMode, SslVerifyMode,
        SslVersion,
    },
    x509::{store::X509StoreBuilder, X509},
};
use tokio::io::{self, AsyncRead, AsyncWrite};

use crate::cmd::TlsArgs;

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

pub fn load_connector(args: TlsArgs) -> Result<SslConnector, Box<dyn std::error::Error>> {
    let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
    builder.set_private_key_file(args.key.unwrap(), SslFiletype::PEM)?;
    builder.set_certificate_chain_file(args.cert.unwrap())?;
    let ca_cert = fs::read_to_string(args.ca.unwrap())?.into_bytes();
    let client_ca_cert = X509::from_pem(&ca_cert)?;
    let mut x509_client_store_builder = X509StoreBuilder::new()?;
    x509_client_store_builder.add_cert(client_ca_cert)?;
    let client_cert_store = x509_client_store_builder.build();
    builder.set_verify_cert_store(client_cert_store)?;
    let mut verify_mode = SslVerifyMode::empty();
    verify_mode.set(SslVerifyMode::PEER, true);
    verify_mode.set(SslVerifyMode::FAIL_IF_NO_PEER_CERT, true);
    builder.set_verify(verify_mode);
    builder.set_session_cache_mode(SslSessionCacheMode::OFF);
    let min_ssl_version_3 = Some(SslVersion::SSL3);
    builder.set_min_proto_version(min_ssl_version_3)?;
    let sslconnector = builder.build();
    Ok(sslconnector)
}

pub fn load_acceptor(args: TlsArgs) -> Result<SslAcceptor, Box<dyn Error>> {
    let mut builder = SslAcceptor::mozilla_modern(SslMethod::tls()).unwrap();
    builder.set_private_key_file(args.key.unwrap(), SslFiletype::PEM)?;
    builder.set_certificate_chain_file(args.cert.unwrap())?;
    let ca_cert = fs::read_to_string(args.ca.unwrap())?.into_bytes();
    let client_ca_cert = X509::from_pem(&ca_cert)?;
    let mut x509_client_store_builder = X509StoreBuilder::new()?;
    x509_client_store_builder.add_cert(client_ca_cert)?;
    let client_cert_store = x509_client_store_builder.build();
    builder.set_verify_cert_store(client_cert_store)?;

    // set options to make sure to validate the peer aka mtls
    let mut verify_mode = SslVerifyMode::empty();
    verify_mode.set(SslVerifyMode::PEER, true);
    verify_mode.set(SslVerifyMode::FAIL_IF_NO_PEER_CERT, true);
    builder.set_verify(verify_mode);

    // may not need to set it to off:
    // https://www.openssl.org/docs/man1.0.2/man3/SSL_CTX_set_session_cache_mode.html
    // https://vincent.bernat.ch/en/blog/2011-ssl-session-reuse-rfc5077
    builder.set_session_cache_mode(SslSessionCacheMode::OFF);
    let min_ssl_version_3 = Some(SslVersion::SSL3);
    builder.set_min_proto_version(min_ssl_version_3)?;
    let acceptor = builder.build();
    Ok(acceptor)
}
