mod sockets_handler;
use sockets_handler::Direction;
use sockets_handler::SocketsHandler;

mod connection_data;
use connection_data::ConnectionData;

mod lua;
use lua::Lua;

mod configuration;
use configuration::Configuration;

use futures::sink::Sink;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::{Future, Stream};
use tokio::sync::mpsc;

fn main() {
    let conf = Configuration::new(
        r#"
		listen="127.0.0.1:12345"
		connect="127.0.0.1:8000"
		[ssl]
		#private_key= "private.key"
		#certificate= "cert.pem"
		"#,
    )
    .unwrap();

    let listener = TcpListener::bind(&conf.listen).unwrap();
    let server = listener
        .incoming()
        .for_each(move |socket| {
            let stream = TcpStream::connect(&conf.connect);
            stream
                .and_then(|stream| {
                    let connection_data = Arc::new(Mutex::new(ConnectionData::new()));
                    let stream_push_connection_data = connection_data.clone();
                    let stream_push_connection_data_finish = connection_data.clone();

                    let (command_tx, command_rx) = mpsc::channel::<Direction<Vec<u8>>>(1000);
                    let sh = SocketsHandler::new(socket, stream);
                    let (network_sender, network_receiver) = sh.split();

                    let lua_embeded = Lua::new(connection_data, command_tx.clone());
                    let lua_thread_handle = lua_embeded.run();
                    let lua_thread = lua_thread_handle.thread().clone();
                    let lua_thread2 = lua_thread_handle.thread().clone();

                    let receiver_future = network_receiver
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
                            Ok(())
                        });

                    let client_to_tcp = command_rx
                        .map_err(|_| ())
                        .and_then(|p| Ok(p))
                        .forward(network_sender.sink_map_err(|_| ()))
                        .then(|_| Ok(()));

                    tokio::spawn(
                        receiver_future
                            .select(client_to_tcp)
                            .map(|_| ())
                            .map_err(|_| ()),
                    );
                    Ok(())
                })
                .map(|_| ())
        })
        .map_err(|_| ());
    tokio::run(server);
}
