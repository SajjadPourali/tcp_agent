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
pub struct SocketsHandler<R, W> {
	server_reader: Option<R>,
	server_writer: Option<W>,
	client_reader: Option<R>,
	client_writer: Option<W>,
	socket_state: SocketState,
	read_buf: Box<[u8]>,
}

impl<R, W> SocketsHandler<R, W>
where
	R: AsyncRead,
	W: AsyncWrite,
{
	pub fn new(server: (R, W), client: (R, W)) -> SocketsHandler<R, W> {
		SocketsHandler {
			server_reader: Some(server.0),
			server_writer: Some(server.1),
			client_reader: Some(client.0),
			client_writer: Some(client.1),
			socket_state: SocketState::AllOpen,
			read_buf: Box::new([0; 2048]),
		}
	}
}

impl<R, W> Stream for SocketsHandler<R, W>
where
	R: AsyncRead,
	W: AsyncWrite,
{
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

impl<R, W> Sink for SocketsHandler<R, W>
where
	R: AsyncRead,
	W: AsyncWrite,
{
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
