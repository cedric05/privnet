use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};

pub enum MayBeTlsStream<R, T> {
    Raw(R),
    Tls(T),
}

impl<R, T> AsyncRead for MayBeTlsStream<R, T>
where
    R: AsyncRead + Unpin,
    T: AsyncRead + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            MayBeTlsStream::Raw(r) => Pin::new(r).poll_read(cx, buf),
            MayBeTlsStream::Tls(r) => Pin::new(r).poll_read(cx, buf),
        }
    }
}

impl<R, T> AsyncWrite for MayBeTlsStream<R, T>
where
    R: AsyncWrite + Unpin,
    T: AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match &mut *self {
            MayBeTlsStream::Raw(_) => todo!(),
            MayBeTlsStream::Tls(_) => todo!(),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            MayBeTlsStream::Raw(_) => todo!(),
            MayBeTlsStream::Tls(_) => todo!(),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            MayBeTlsStream::Raw(_) => todo!(),
            MayBeTlsStream::Tls(_) => todo!(),
        }
    }
}
