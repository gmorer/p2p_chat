#[cfg(feature = "full")]
use std::str;
use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{Arc, Mutex},
};
use std::path::Path;
use std::ffi::OsStr;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::header::{HeaderValue, UPGRADE};
use hyper::server::conn::AddrStream;

use futures::channel::mpsc::UnboundedSender;
// use futures_util::stream::StreamExt;
use tungstenite::protocol::Message;

use crossplatform::id::Id;

mod websocket;
mod process;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, (Id, Tx)>>>;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// const ADDR: &str = "127.0.0.1:8088";
const ADDR: &str = "[::1]:8088";
const STATIC_FOLDER: &str = "./static/";

async fn shutdown_signal() {
	// Wait for the CTRL+C signal
	tokio::signal::ctrl_c()
		.await
		.expect("failed to install CTRL+C signal handler");
}

pub async fn send_static(req: Request<Body>) -> Result<Response<Body>> {
	let uri = match &(req.uri().to_string())[..] {
		"/" => "index.html".to_string(),
		uri => uri.to_string()
	};
	let mime = match Path::new(&uri).extension().and_then(OsStr::to_str).unwrap() {
		"html" => "text/html",
		"js" => "application/javascript",
		"wasm" => "application/wasm",
		_ => ""
	};
	// TODO: Range header
	let file = match File::open(format!("{}/{}", STATIC_FOLDER, uri)).await {
		Ok(file) => file,
		Err(_) => File::open(format!("{}/{}", STATIC_FOLDER, "index.html")).await.expect(&format!("cannot find index.html in {}", STATIC_FOLDER))
	};
	let stream = FramedRead::new(file, BytesCodec::new());
	let body = Body::wrap_stream(stream);
	let builder = Response::builder()
		.header(hyper::header::CONTENT_TYPE, mime)
		.status(StatusCode::OK);

	Ok(builder.body(body)?)
}

/// Our server HTTP handler to initiate HTTP upgrades.
async fn handler(peers: PeerMap, addr: SocketAddr, req: Request<Body>) -> Result<Response<Body>> {
	println!("{:?}", req);
	if req.headers().get(UPGRADE) == Some(&HeaderValue::from_static("websocket")) {
		websocket::handler(peers, addr, req).await
	} else { send_static(req).await }
}

#[tokio::main]
async fn main() -> Result<()> {
	// let data = WebSocketData { data: "Hello World!".to_string() };
	// println!("{}", data.data);
	
	let peers = PeerMap::new(Mutex::new(HashMap::new()));
	
	let addr = ADDR.parse().expect("Invalid server address");
	let new_service = make_service_fn(move |conn: &AddrStream| {
			// println!("{:?}", conn.remote_addr());
			let addr = conn.remote_addr();
			let peers = peers.clone();
			async move {
				Ok::<_, hyper::Error>(service_fn(move |req| handler(peers.clone(), addr, req)))
			}
		});

	let server = Server::bind(&addr).serve(new_service);
	let graceful = server.with_graceful_shutdown(shutdown_signal());
	if let Err(e) = graceful.await {
		eprintln!("server error: {}", e);
	}
	Ok(())
}