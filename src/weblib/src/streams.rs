use std::net::SocketAddr;
use crossplatform::proto::WebSocketData;

use crate::{ log, console_log };
use crate::webrtc::RTCSocket;

pub enum Data {
	WsData(WebSocketData),
	RtcData(String)
}
#[derive(Clone)]
pub enum Socket {
	WebSocket(web_sys::WebSocket),
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
			(Some(Socket::WebSocket(socket)), Data::WsData(data)) => 
				{ if let Err(e) = socket.send_with_u8_array(data.into_u8().expect("error while transforming").as_slice()) {
					console_log!("Error while sending to server: {:?}", e)
				}},
			(Some(Socket::WebRTC(socket)), Data::RtcData(data)) =>
				socket.send(&data),
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
pub struct Sockets {
	pub server: Pstream,
	pub right: Pstream,
	// pub dright: Option<Pstream>,
	pub left: Pstream,
	// pub dleft: Option<Pstream>
	pub tmp: Pstream
}

impl Sockets {
	pub fn default() -> Self {
		Sockets {
			server: Pstream { state: State::Disconnected(None), socket: None },
			// server: Some(Pstream::from_ws(server_ws)),
			right: Pstream { state: State::Disconnected(None), socket: None},
			// dright: None,
			left: Pstream { state: State::Disconnected(None), socket: None},
			tmp: Pstream { state: State::Disconnected(None), socket: None}
			// dleft: None
		}
	}
}