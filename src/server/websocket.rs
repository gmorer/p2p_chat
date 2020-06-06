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


use crate::PeerMap;
use crate::Result;

async fn upgrade(peers: PeerMap, addr: SocketAddr, upgraded: Upgraded) {
	// transform hyper upgraded to tungstenit stream
	let ws_stream = tokio_tungstenite::WebSocketStream::from_raw_socket(
		upgraded,
		tokio_tungstenite::tungstenite::protocol::Role::Server,
		None,
	).await;
	// create multithread stream to keep it in the mutex
	let (tx, rx) = unbounded();
	peers.lock().unwrap().insert(addr, tx);
	let (ws_sender, ws_receiver) = ws_stream.split();
	// create new client
	
	// broadcast_incoming stop when the stream stop
	let broadcast_incoming = ws_receiver.try_for_each(|msg| {
		println!(
			"Received a message from {}: {}",
			addr,
			msg.to_text().unwrap()
		);
		let peers = peers.lock().unwrap();
		
		// We want to broadcast the message to everyone except ourselves.
		let broadcast_recipients = peers
			.iter()
			.filter(|(peer_addr, _)| peer_addr != &&addr)
			.map(|(_, ws_sink)| ws_sink);
		
		for recp in broadcast_recipients {
			// sending to the unbound stream will be forwarded after to the real one
			recp.unbounded_send(msg.clone()).unwrap();
		}
		
		future::ok(())
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