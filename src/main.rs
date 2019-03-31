use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::runtime::Runtime as TokioRuntime;
use toml;

mod connection_data;
mod lua;
use connection_data::ConnectionData;
use lua::Lua;

fn main() {
	let conf = Configuration::new(
		r#"
		listen="127.0.0.1:12345"
		#connect="127.0.0.1:2222"
		connect="198.143.180.42:22"
		[ssl]
		#private_key= "private.key"
		#certificate= "cert.pem" # cert forge planned
		"#,
	)
	.unwrap();

	// let _runtime = Arc::new(SocketRuntime::new(&conf));

	let mut rt = TokioRuntime::new().unwrap();

	let listener = TcpListener::bind(&conf.listen).expect("unable to bind TCP listener");
	let server = listener
		.incoming()
		.map_err(|e| eprintln!("accept failed = {:?}", e))
		.for_each(move |socket| {
			TcpStream::connect(&conf.connect)
				.and_then(|target| {
					let (socket_reader, socket_writer) = socket.split();
					let (target_reader, target_writer) = target.split();
					let connection_data = Arc::new(Mutex::new(ConnectionData::new()));
					let stream_push_connection_data = connection_data.clone();

					let x0 = Arc::new(Mutex::new(socket_writer));
					let x1 = Arc::new(Mutex::new(target_writer));

					let stream_push_connection_data_finish = connection_data.clone();

					let lua_embeded = Lua::new(connection_data, (x0.clone(), x1.clone()));
					
					let lua_thread_handle = lua_embeded.run();
					let lua_thread = lua_thread_handle.thread().clone();
					let lua_thread2 = lua_thread_handle.thread().clone();
					let req_handler = reader_handler(socket_reader, target_reader)
						.for_each(move |data| {
							let mut connection_data = stream_push_connection_data.lock().unwrap();
							lua_thread.unpark();
							connection_data.push_original(data);
							Ok(())
						})
						.and_then(move |_| {
							let mut connection_data =
								stream_push_connection_data_finish.lock().unwrap();
							lua_thread2.unpark();
							connection_data.push_original(Direction::None);
							// lua_thread_handle.join().unwrap();
							Ok(())
						});
					tokio::spawn(req_handler);
					// lua_thread_handle.join().unwrap();
					Ok(())
				})
				.map_err(|_| ())
		});

	rt.spawn(server);
	rt.shutdown_on_idle().wait().unwrap();
}

#[derive(Debug)]
pub enum Direction<T> {
	Out(T),
	In(T),
	NotReady,
	None,
}

struct SockerWriter<W> {
	source: Option<W>,
	destination: Option<W>,
	data: Arc<Mutex<ConnectionData>>,
}

// fn writer_handler<W>(data: Arc<Mutex<ConnectionData>>, source: W, destination: W) -> SockerWriter<W>
// where
// 	W: AsyncWrite,
// {
// 	SockerWriter {
// 		source: Some(source),
// 		destination: Some(destination),
// 		data: data.clone(),
// 	}
// }

impl<W> futures::Future for SockerWriter<W>
where
	W: AsyncWrite,
{
	type Item = ();
	type Error = ();
	fn poll(&mut self) -> Result<Async<()>, ()> {
		let mut cd = self.data.lock().unwrap();
		match cd.get_modified(2048) {
			Direction::In(data) => {
				self.source.as_mut().unwrap().write_all(&data).unwrap();
				Ok(Async::NotReady)
			}
			Direction::Out(data) => {
				self.destination.as_mut().unwrap().write_all(&data).unwrap();
				Ok(Async::NotReady)
			}
			Direction::NotReady => Ok(Async::NotReady),
			Direction::None => Ok(Async::Ready(())),
		}
		//		unimplemented!()
	}
}

struct SocketReader<R> {
	source: Option<R>,
	destination: Option<R>,
	buf: Box<[u8]>,
}

fn reader_handler<R>(source: R, destination: R) -> SocketReader<R>
where
	R: AsyncRead,
{
	SocketReader {
		source: Some(source),
		destination: Some(destination),
		buf: Box::new([0; 2048]),
	}
}

impl<R> futures::Stream for SocketReader<R>
where
	R: AsyncRead,
{
	type Item = Direction<Vec<u8>>;
	type Error = ();
	fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
		let src = self.source.as_mut().unwrap();
		let dst = self.destination.as_mut().unwrap();

		match dst.poll_read(&mut self.buf).map_err(|_| ()) {
			Ok(futures::Async::Ready(t)) => {
				if t == 0 {
					Ok(Async::Ready(None))
				} else {
					Ok(Async::Ready(Some(Direction::In(self.buf[..t].to_vec()))))
				}
			}
			Ok(futures::Async::NotReady) => match src.poll_read(&mut self.buf).map_err(|_| ()) {
				Ok(futures::Async::Ready(t)) => {
					if t == 0 {
						Ok(Async::Ready(None))
					} else {
						Ok(Async::Ready(Some(Direction::Out(self.buf[..t].to_vec()))))
					}
				}
				Ok(futures::Async::NotReady) => Ok(futures::Async::NotReady),
				Err(e) => Err(From::from(e)),
			},
			Err(e) => Err(From::from(e)),
		}
	}
}

// struct SocketHandler<R> {
// 	reader: Option<R>,
// 	buf: Box<[u8]>,
// }

// fn socket_handler<R>(reader: R) -> SocketHandler<R>
// where
// 	R: AsyncRead,
// {
// 	SocketHandler {
// 		reader: Some(reader),
// 		buf: Box::new([0; 40]),
// 	}
// }

// impl<R> futures::Stream for SocketHandler<R>
// where
// 	R: AsyncRead,
// {
// 	type Item = usize;
// 	type Error = ();
// 	fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
// 		let reader = self.reader.as_mut().unwrap();

// 		let n = match reader.poll_read(&mut self.buf).map_err(|_| ()) {
// 			Ok(futures::Async::Ready(t)) => t,
// 			Ok(futures::Async::NotReady) => return Ok(futures::Async::NotReady),
// 			Err(e) => return Err(From::from(e)),
// 		};

// 		if n == 0 {
// 			Ok(Async::Ready(None))
// 		} else {
// 			Ok(Async::Ready(Some(n)))
// 		}
// 	}
// }

// use futures::{Future, Poll};
// use {AsyncRead, AsyncWrite};
// pub struct SocketHandler<R, W> {
// 	reader: Option<R>,
// 	writer: Option<W>,
// 	runtime: Arc<SocketRuntime>,
// 	buf: Box<[u8]>,
// }

// fn socket_handler<R, W>(reader: R, writer: W, runtime: Arc<SocketRuntime>) -> SocketHandler<R, W>
// where
// 	R: AsyncRead,
// 	W: AsyncWrite,
// {
// 	SocketHandler {
// 		reader: Some(reader),
// 		writer: Some(writer),
// 		runtime: runtime,
// 		buf: Box::new([0; 2048]),
// 	}
// }

// impl<R, W> Future for SocketHandler<R, W>
// where
// 	R: AsyncRead,
// 	W: AsyncWrite,
// {
// 	type Item = ();
// 	type Error = io::Error;

// 	fn poll(&mut self) -> Poll<(), io::Error> {

// 		// use rlua::{Error, Lua, Result, String};
// 		// use std::ffi::OsStr;
// 		// use std::fs;
// 		// use std::os::unix::ffi::OsStrExt;

// 		// let lua = Lua::new();
// 		// lua.context(|ctx| {
// 		// 	ctx.scope(|ctx| {
// 		// 		ctx.create_function_mut(move |ctx, ()| {
// 		// 			let reader = self.reader.as_mut().unwrap();
// 		// 			futures::try_ready!(reader.poll_read(&mut self.buf));
// 		// 			Ok(())
// 		// 		})
// 		// 	});
// 		// });

// 		// let reader = self.reader.as_mut().unwrap();
// 		//	futures::try_ready!(reader.poll_read(&mut self.buf));

// 		return Ok(futures::Async::Ready(()));
// 	}
// }

// struct SocketRuntime {
// 	ssl_acceptor: Option<std::sync::Arc<openssl::ssl::SslAcceptor>>,
// }

// impl SocketRuntime {
// 	fn new(conf: &Configuration) -> Self {
// 		SocketRuntime {
// 			ssl_acceptor: match &conf.ssl {
// 				Some(ssl) => {
// 					let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
// 					builder
// 						.set_private_key_file(&ssl.private_key, SslFiletype::PEM)
// 						.unwrap();
// 					builder
// 						.set_certificate_chain_file(&ssl.certificate)
// 						.unwrap();
// 					Some(Arc::new(builder.build()))
// 				}
// 				None => None,
// 			},
// 		}
// 	}
// }

// #[derive(Deserialize)]
// struct SslAcceptBuilder {
// 	private_key: String,
// 	certificate: String,
// }

#[derive(Deserialize)]
struct Configuration {
	listen: std::net::SocketAddr,
	connect: std::net::SocketAddr,
	// ssl: Option<SslAcceptBuilder>,
	//	acceptor: std::sync::Arc<openssl::ssl::SslAcceptor>,
}

impl Configuration {
	fn new(conf_str: &str) -> Result<Self, toml::de::Error> {
		toml::from_str(conf_str)
	}
}

// struct Intercept {
// 	//conf: configuration
// }

// impl Intercept {
// 	fn new() -> Self {
// 		Intercept {}
// 	}
// }

// impl Future for Intercept
// // where
// // 	R: AsyncRead,
// // 	W: AsyncWrite,
// {
// 	type Item = ();
// 	type Error = io::Error;
// 	fn poll(&mut self) -> Poll<(), io::Error> {
// 		unimplemented!()
// 	}
// }
