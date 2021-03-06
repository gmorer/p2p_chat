#[cfg(feature = "full")]
use std::str;
use std::env;
use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{Arc, Mutex}
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
const ADDR_KEY: &str = "P2P_ADDR";
const ADDR_DFL: &str = "127.0.0.1";
const PORT_KEY: &str = "PORT";
const PORT_DFL: &str = "8088";
const STATIC_FOLDER_KEY: &str = "P2P_STATIC_FILES";
const STATIC_FOLDER_DFL: &str = "./static/";

fn log_err<T: core::fmt::Display>(arg: std::result::Result<(), T>) {
	if let Err(e) = arg {
		eprintln!("Unhandled error: {}", e);
	}
}

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
        "css" => "text/css",
		_ => {
            println!("no mime type for {}", uri);
            ""
        }
	};
	let static_folder = env::var(STATIC_FOLDER_KEY).unwrap_or(STATIC_FOLDER_DFL.to_string());
	// TODO: Range header
	let file = match File::open(format!("{}/{}", static_folder, uri)).await {
		Ok(file) => file,
		Err(_) => File::open(format!("{}/{}", static_folder, "index.html")).await.expect(&format!("cannot find index.html in {}", static_folder))
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
    let res = if req.headers().get(UPGRADE) == Some(&HeaderValue::from_static("websocket")) {
        println!("======incomming======");
        println!("{:?}", req.headers());
        websocket::handler(peers, addr, req).await
    } else { send_static(req).await };
    println!("======outgoing======");

    println!("{:?}", res.as_ref().unwrap().headers());
    res
}

#[tokio::main]
async fn main() -> Result<()> {
	// let data = WebSocketData { data: "Hello World!".to_string() };
	// println!("{}", data.data);
	
	let peers = PeerMap::new(Mutex::new(HashMap::new()));
	let addr = env::var(ADDR_KEY).unwrap_or(ADDR_DFL.to_string());
	let port = env::var(PORT_KEY).unwrap_or(PORT_DFL.to_string());
	let addr = format!("{}:{}", addr, port);
	// let addr = format!("{}:{}", addr, port).parse().expect("Invalid server address");
	let new_service = make_service_fn(move |conn: &AddrStream| {
			// println!("{:?}", conn.remote_addr());
			let addr = conn.remote_addr();
			let peers = peers.clone();
			async move {
				Ok::<_, hyper::Error>(service_fn(move |req| handler(peers.clone(), addr, req)))
			}
		});

	let server = Server::bind(&addr.parse().expect(&format!("Invalid server address: {}", addr))).serve(new_service);
	let graceful = server.with_graceful_shutdown(shutdown_signal());
	println!("Listening on {}", addr);
	if let Err(e) = graceful.await {
		eprintln!("server error: {}", e);
	}
	Ok(())
}