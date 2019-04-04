use tokio::prelude::{AsyncRead, AsyncWrite};

pub struct ProxyParser<R, W> {
	socket_reader: Option<R>,
	socket_writer: Option<W>,
}

impl<R, W> ProxyParser<R, W>
where
	R: AsyncRead,
	W: AsyncWrite,
{
	pub fn new(i: (R, W)) -> Self {
		Self {
			socket_reader: Some(i.0),
			socket_writer: Some(i.1),
		}
	}
}

impl<R, W> std::io::Read for ProxyParser<R, W> {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		unimplemented!()
	}
}

impl<R, W> tokio::prelude::AsyncRead for ProxyParser<R, W> {}

impl<R, W> std::io::Write for ProxyParser<R, W> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		unimplemented!()
	}
	fn flush(&mut self) -> std::io::Result<()> {
		unimplemented!()
	}
}

impl<R, W> tokio::prelude::AsyncWrite for ProxyParser<R, W> {
	fn shutdown(&mut self) -> Result<tokio::prelude::Async<()>, std::io::Error> {
		unimplemented!()
	}
}
