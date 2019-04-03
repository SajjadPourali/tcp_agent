#[derive(Debug, Clone)]
pub enum Direction<T> {
    Out(T),
    In(T),
    NotReady,
    None,
}
#[derive(PartialEq)]
enum SocketState {
    ServerClosed,
    ClientClosed,
    AllOpen,
    AllClosed,
}

use tokio::prelude::{AsyncRead, AsyncWrite, Poll, Sink, Stream};
pub struct SocketsHandler {
    server_reader: Option<tokio::io::ReadHalf<tokio::net::TcpStream>>,
    server_writer: Option<tokio::io::WriteHalf<tokio::net::TcpStream>>,
    client_reader: Option<tokio::io::ReadHalf<tokio::net::TcpStream>>,
    client_writer: Option<tokio::io::WriteHalf<tokio::net::TcpStream>>,
    socket_state: SocketState,
    read_buf: Box<[u8]>,
}

impl SocketsHandler {
    pub fn new(server: tokio::net::TcpStream, client: tokio::net::TcpStream) -> SocketsHandler {
        let (sr, sw) = server.split();
        let (cr, cw) = client.split();
        SocketsHandler {
            server_reader: Some(sr),
            server_writer: Some(sw),
            client_reader: Some(cr),
            client_writer: Some(cw),
            socket_state: SocketState::AllOpen,
            read_buf: Box::new([0; 2048]),
        }
    }
}

impl Stream for SocketsHandler {
    type Item = Direction<Vec<u8>>;
    type Error = tokio::io::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let server = self.server_reader.as_mut().unwrap();
        let client = self.client_reader.as_mut().unwrap();

        match client.poll_read(&mut self.read_buf) {
            Ok(futures::Async::Ready(t)) => {
                if t == 0 {
                    match self.socket_state {
                        SocketState::AllOpen => {
                            self.socket_state = SocketState::ClientClosed;
                            Ok(futures::Async::NotReady)
                        }
                        SocketState::ClientClosed => Ok(futures::Async::NotReady),
                        SocketState::ServerClosed => {
                            self.socket_state = SocketState::AllClosed;
                            Ok(futures::Async::Ready(None))
                        }
                        SocketState::AllClosed => unreachable!(),
                    }
                } else {
                    Ok(futures::Async::Ready(Some(Direction::In(
                        self.read_buf[..t].to_vec(),
                    ))))
                }
            }
            Ok(futures::Async::NotReady) => match server.poll_read(&mut self.read_buf) {
                Ok(futures::Async::Ready(t)) => {
                    if t == 0 {
                        match self.socket_state {
                            SocketState::AllOpen => {
                                self.socket_state = SocketState::ServerClosed;
                                Ok(futures::Async::NotReady)
                            }
                            SocketState::ClientClosed => {
                                self.socket_state = SocketState::AllClosed;
                                Ok(futures::Async::Ready(None))
                            }
                            SocketState::ServerClosed => Ok(futures::Async::NotReady),
                            SocketState::AllClosed => unreachable!(),
                        }
                    } else {
                        Ok(futures::Async::Ready(Some(Direction::Out(
                            self.read_buf[..t].to_vec(),
                        ))))
                    }
                }
                Ok(futures::Async::NotReady) => Ok(futures::Async::NotReady),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    }
}

impl Sink for SocketsHandler {
    type SinkItem = Direction<Vec<u8>>;
    type SinkError = tokio::io::Error;
    fn start_send(
        &mut self,
        item: Self::SinkItem,
    ) -> futures::StartSend<Self::SinkItem, Self::SinkError> {
        let src = self.server_writer.as_mut().unwrap();
        let dst = self.client_writer.as_mut().unwrap();
        match item.clone() {
            Direction::Out(data) => match dst.poll_write(&data) {
                Ok(futures::Async::Ready(_)) => Ok(futures::AsyncSink::Ready),
                Ok(futures::Async::NotReady) => Ok(futures::AsyncSink::NotReady(item)),
                Err(x) => Err(x),
            },
            Direction::In(data) => match src.poll_write(&data) {
                Ok(futures::Async::Ready(_)) => Ok(futures::AsyncSink::Ready),
                Ok(futures::Async::NotReady) => Ok(futures::AsyncSink::NotReady(item)),
                Err(x) => Err(x),
            },
            Direction::NotReady => unreachable!(),
            Direction::None => unreachable!(),
        }
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.server_writer.as_mut().unwrap().poll_flush().unwrap();
        self.client_writer.as_mut().unwrap().poll_flush().unwrap();
        Ok(futures::Async::Ready(()))
    }
}
