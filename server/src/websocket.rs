use futures_util::{
	future, pin_mut,
	stream::TryStreamExt,
	StreamExt,
};
use std::net::SocketAddr;
use futures::channel::mpsc::unbounded;
use hyper::upgrade::Upgraded;
use hyper::{Body, Request, Response, StatusCode};
use headers::HeaderMapExt;
use crossplatform::proto_ws::WebSocketData;
use crossplatform::id::Id;
use tungstenite::Message;
use tungstenite::error::Error;
use crate::process::process;

use crate::PeerMap;
use crate::Result;
use crate::log_err;

async fn upgrade(peers: PeerMap, addr: SocketAddr, upgraded: Upgraded) {
	// transform hyper upgraded to tungstenit stream
	let ws_stream = tokio_tungstenite::WebSocketStream::from_raw_socket(
		upgraded,
		tokio_tungstenite::tungstenite::protocol::Role::Server,
		None,
	).await;
	// create multithread stream to keep it in the mutex
	let (tx, rx) = unbounded();
	let id = Id::new(rand::random(), rand::random());
	peers.lock().unwrap().insert(addr, (id, tx));
	let (ws_sender, ws_receiver) = ws_stream.split();
	// create new client
	
	// broadcast_incoming stop when the stream stop
	let broadcast_incoming = ws_receiver.try_for_each(|msg| {
		let msg = match WebSocketData::from_u8(msg.into_data()) {
			Ok(msg) => msg,
			Err(e) => {
				eprintln!("Socket: error while parsing incomming message: {}", e);
				return future::err(Error::Protocol(std::borrow::Cow::Borrowed("Invalid protocol")));
			}
		};
		println!("Received a message from {}: {:?}", addr, msg);
		
		// process the msg
		let rsp = process(addr, msg, &peers);
		
		if let Some(rsp) = rsp {
			let rsp = match rsp.into_u8() {
				Ok(rsp) => Message::binary(rsp),
				Err(e) => {
					eprintln!("Error while creating data from msg: {}", e);
					return future::err(Error::Protocol(std::borrow::Cow::Borrowed("Internal Error")));
				}
			};
			// TODO: remove those warning
			match peers.lock().unwrap().get(&addr) {
				Some((_id, sender)) => log_err(sender.unbounded_send(rsp)),
				None => {
					eprintln!("Cannot a reply to a phantom");
					return future::err(Error::Protocol(std::borrow::Cow::Borrowed("Internal Error")));
				}
			};
		}
		future::ok(())
		
		// We want to broadcast the message to everyone except ourselves.
		// future::err(Error::Protocol(std::borrow::Cow::Borrowed("lol")))
	});
	// forwarding everything comming from the unbound stream to the real stream
	let receive_from_others = rx.map(Ok).forward(ws_sender);
	pin_mut!(broadcast_incoming, receive_from_others);
	future::select(broadcast_incoming, receive_from_others).await;

	println!("{} disconnected", &addr);
	peers.lock().unwrap().remove(&addr);
}

pub async fn handler(peers: PeerMap, addr: SocketAddr, req: Request<Body>) -> Result<Response<Body>> {
	// Websocket creation
	let key = match req.headers().typed_get::<headers::SecWebsocketKey>() {
		Some(key) => key,
		None => return crate::send_static(req).await
	};
    // spawn task that will be trigerd after the HTML response
    println!("Upgrade starting...");
	tokio::task::spawn(async move {
		// transform the body into a future
		match req.into_body().on_upgrade().await {
			Ok(upgraded) => {
				eprintln!("updrage receive");
				upgrade(peers, addr, upgraded).await;
			}
			Err(e) => eprintln!("upgrade error: {}", e),
		}
	});
	// Manual handshake response with headers crate
	let mut rsp = Response::builder()
		.status(StatusCode::SWITCHING_PROTOCOLS)
		.body(Body::empty())?;
	rsp.headers_mut().typed_insert(headers::Upgrade::websocket());
	rsp.headers_mut().typed_insert(headers::Connection::upgrade());
	rsp.headers_mut().typed_insert(headers::SecWebsocketAccept::from(key));
	Ok(rsp)
}