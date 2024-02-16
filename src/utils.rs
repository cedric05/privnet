use std::path::PathBuf;
use std::{fs, io::BufReader, net::TcpListener, sync::Arc};

use rustls::crypto::{aws_lc_rs as provider, CryptoProvider};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    server::WebPkiClientVerifier,
    RootCertStore,
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

pub fn load_private_key(filename: &PathBuf) -> PrivateKeyDer<'static> {
    let keyfile = fs::File::open(filename).expect("cannot open private key file");
    let mut reader = BufReader::new(keyfile);

    loop {
        match rustls_pemfile::read_one(&mut reader).expect("cannot parse private key .pem file") {
            Some(rustls_pemfile::Item::Pkcs1Key(key)) => return key.into(),
            Some(rustls_pemfile::Item::Pkcs8Key(key)) => return key.into(),
            Some(rustls_pemfile::Item::Sec1Key(key)) => return key.into(),
            None => break,
            _ => {}
        }
    }

    panic!(
        "no keys found in {:?} (encrypted keys not supported)",
        filename
    );
}

pub fn load_certs(filename: &PathBuf) -> Vec<CertificateDer<'static>> {
    let certfile = fs::File::open(filename).expect("cannot open certificate file");
    let mut reader = BufReader::new(certfile);
    rustls_pemfile::certs(&mut reader)
        .map(|result| result.unwrap())
        .collect()
}

pub fn get_server_config(args: &TlsArgs) -> Arc<rustls::ServerConfig> {
    let roots = load_certs(args.ca.as_ref().unwrap());
    let mut client_auth_roots = RootCertStore::empty();
    for root in roots {
        client_auth_roots.add(root).unwrap();
    }

    let client_auth = WebPkiClientVerifier::builder(client_auth_roots.into())
        .build()
        .unwrap();

    let suites = provider::ALL_CIPHER_SUITES.to_vec();

    let versions = rustls::ALL_VERSIONS.to_vec();

    let certs = load_certs(args.cert.as_ref().expect("--certs option missing"));
    let privkey = load_private_key(args.key.as_ref().expect("--key option missing"));

    let mut config = rustls::ServerConfig::builder_with_provider(
        CryptoProvider {
            cipher_suites: suites,
            ..provider::default_provider()
        }
        .into(),
    )
    .with_protocol_versions(&versions)
    .expect("inconsistent cipher-suites/versions specified")
    .with_client_cert_verifier(client_auth)
    .with_single_cert_with_ocsp(certs, privkey, vec![])
    .expect("bad certificates/private key");

    config.key_log = Arc::new(rustls::KeyLogFile::new());
    Arc::new(config)
}

/// Build a `ClientConfig` from our arguments
pub fn get_client_config(args: &TlsArgs) -> Arc<rustls::ClientConfig> {
    let mut root_store = RootCertStore::empty();

    let cafile = args.ca.as_ref().unwrap();

    let certfile = fs::File::open(cafile).expect("Cannot open CA file");
    let mut reader = BufReader::new(certfile);
    root_store.add_parsable_certificates(
        rustls_pemfile::certs(&mut reader).map(|result| result.unwrap()),
    );

    let suites = provider::DEFAULT_CIPHER_SUITES.to_vec();

    let versions = rustls::DEFAULT_VERSIONS.to_vec();

    let config = rustls::ClientConfig::builder_with_provider(
        CryptoProvider {
            cipher_suites: suites,
            ..provider::default_provider()
        }
        .into(),
    )
    .with_protocol_versions(&versions)
    .expect("inconsistent cipher-suite/versions selected")
    .with_root_certificates(root_store);

    let certs = load_certs(args.cert.as_ref().unwrap());
    let key = load_private_key(args.key.as_ref().unwrap());
    let config = config
        .with_client_auth_cert(certs, key)
        .expect("invalid client auth certs/key");

    Arc::new(config)
}
