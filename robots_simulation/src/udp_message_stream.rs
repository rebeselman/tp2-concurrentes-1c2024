use actix::prelude::*;
use tokio::net::UdpSocket;
use tokio_stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

pub struct UdpMessageStream {
    socket: Arc<UdpSocket>,
}

impl Stream for UdpMessageStream {
    type Item = io::Result<(usize, Vec<u8>, SocketAddr)>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = vec![0; 1024];
        let mut read_buf = tokio::io::ReadBuf::new(&mut buf);
        let socket = &self.get_mut().socket;
        match socket.poll_recv_from(cx, &mut read_buf) {
            Poll::Ready(Ok(addr)) => Poll::Ready(Some(Ok((read_buf.filled().len(), buf, addr)))),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl UdpMessageStream {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self { socket }
    }
}