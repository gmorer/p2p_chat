use std::net::SocketAddr;
use crossplatform::proto_ws::WebSocketData;

use crate::{ log, console_log };
use crate::webrtc::RTCSocket;
use crate::websocket;
use crate::p2p::Network;

// Do we need this file ?

pub enum Data {
	WsData(WebSocketData),
	RtcData(String)
}
#[derive(Clone)]
pub enum Socket {
	WebSocket(websocket::WebSocket),
	WebRTC(RTCSocket)
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum State {
	Disconnected(Option<u64>), // should this exist
	Connected(u64),
	Locked(SocketAddr),
	Waiting(u64),
}

#[derive(Clone)]
pub struct Pstream {
	pub state: State,
	pub socket: Option<Socket>, // remove the public maybe
}

impl Pstream {
	pub fn send(&self, data: Data) {
		match self.state {
			State::Disconnected(x) => { console_log!("Cannot send, the client is disconnected since {:?}", x); return },
			State::Waiting(x) => { console_log!("Cannot send, we are waiting for answer since {}", x); return }
			_ => ()
		};
		match (&self.socket, &data) {
			(Some(Socket::WebSocket(socket)), Data::WsData(data)) => socket.send(data),
			(Some(Socket::WebRTC(socket)), Data::RtcData(data)) => socket.send(data.as_str().as_bytes()),
			_ =>
				console_log!("Invalid data type for websocket")
		};
	}

	// Use those to know if an object is conencted or waiting, no the option
	pub fn is_connected(&self) -> bool {
		match self.state {
			State::Connected(_) => {
				if self.socket.is_none() {
					console_log!("Socket connected but object is none");
				}
				true
			},
			_ => false
		}
	}

	pub fn is_disconnected(&self) -> bool {
		match self.state {
			State::Disconnected(_) => true,
			_ => false
		}
	}

	pub fn is_waiting(&self) -> bool {
		match self.state {
			State::Waiting(_) => {
				if self.socket.is_none() {
					console_log!("socket waiting but object is none");
				}
				true
			},
			_ => false
		}
	}

	pub fn is_locked(&self, paddr: Option<SocketAddr>) -> bool {
		match (paddr, self.state) {
			(Some(paddr), State::Locked(addr)) => paddr == addr,
			(None, State::Locked(_)) => true,
			_ => false
		}
	}
}

// TODO all mutex
pub struct Sockets<'a> {
	pub server: Pstream,
	pub network: Option<Network<'a>>,
	pub tmp: Pstream // should be in Network
	// TODO: Multiples tmp?
}

impl<'a> Sockets<'a> {
	pub fn default() -> Self {
		Sockets {
			server: Pstream { state: State::Disconnected(None), socket: None },
			// server: Some(Pstream::from_ws(server_ws)),
			// dright: None,
			tmp: Pstream { state: State::Disconnected(None), socket: None},
			network: None,
			// dleft: None
		}
	}
}