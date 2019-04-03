use toml;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Configuration {
	pub listen: std::net::SocketAddr,
	pub connect: std::net::SocketAddr,
	// ssl: Option<SslAcceptBuilder>,
	//	acceptor: std::sync::Arc<openssl::ssl::SslAcceptor>,
}

impl Configuration {
	pub fn new(path: &str) -> Result<Self, toml::de::Error> {
		toml::from_str(&std::fs::read_to_string(path).unwrap())
	}
}
