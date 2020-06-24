use wasm_bindgen::prelude::*;
use web_sys::{ RtcPeerConnection, window, BinaryType };
use wasm_bindgen::JsCast;
use std::net::SocketAddr;
use protocols::WebSocketData;

use crate::cb::CB;
use crate::{ log, console_log, Sender };
use crate::html::{ MESSAGE_FIELD_ID, BUTTON_SEND_MESSAGE, get_input_value, set_input_value  };
use crate::webrtc::{ create_rtc, incoming_offer, incomming_answer, incomming_ice_candidate };

#[derive(Debug)]
#[allow(dead_code)]
pub enum Branch {
	Server,
	Right,
	DRight,
	Left,
	Dleft
}

fn reconnect_server(socks: &mut Sockets, cb: &CB) {
	let socket_url = format!(
		"ws://{}",
		window().expect("Cannot get the window object").location().host().expect("cannot get the url")
	);
	console_log!("window location: {} ", socket_url);
	match web_sys::WebSocket::new(&socket_url) {
		Ok(ws) => {
			ws.set_binary_type(BinaryType::Arraybuffer);
			ws.set_onopen(Some(cb.connected_from_server.as_ref().unchecked_ref()));
			ws.set_onclose(Some(cb.disconnect_from_server.as_ref().unchecked_ref()));
			ws.set_onmessage(Some(cb.message_from_server.as_ref().unchecked_ref()));
			socks.server.socket = Some(Socket::WebSocket(ws));
		}
		Err(e) => console_log!("Error while connecting the server socket: {:?}", e)
	};
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
	Verification, // Created once evry xtime
	Disconnect(Branch),
	Connected(Branch),
	ServerMessage(Branch, WebSocketData), // TODO: Message struct
	Html(String, JsValue) // event from html
}

impl Event {
	pub async fn execute(self, _sender: Sender, socks: &mut Sockets, cb: &CB) {
		match self {
			Event::Verification => console_log!("Getting verification"),
			Event::Disconnect(branch) => Event::disconnect(socks, cb, branch),
			Event::Connected(branch) => Event::connected(socks, branch).await,
			Event::ServerMessage(branch, msg) => Event::server_msg(socks, msg, branch).await,
			Event::Html(id, msg) => Event::html(socks, id, msg)
		};
	}

	async fn server_msg(socks: &mut Sockets, msg: WebSocketData, branch: Branch) {
		match msg {
			WebSocketData::OfferSDP(sdp, Some(addr)) => {
				incoming_offer(socks, &sdp, addr).await;
				// console_log!("receiveing OfferSDP: {} {:?}", sdp, addr);
				// let rsp = WebSocketData::AnswerSDP("pong".to_string(), addr);
				// socks.server.send(Data::WsData(rsp));
			},
			WebSocketData::AnswerSDP(sdp, addr) => {
				// console_log!("receiveing AnswerSDP: {} {:?}", sdp, addr);
				incomming_answer(socks, &sdp, addr).await;
				// let rsp = WebSocketData::IceCandidate("data".to_string(), addr);
				// socks.server.send(Data::WsData(rsp));
			},
			WebSocketData::IceCandidate(candidate, addr) => {
				// console_log!("receiveing IceCandidate: {:?} {:?}", candidate, addr);
				incomming_ice_candidate(socks, &candidate, addr).await;
			},
			_ => console_log!("Cannot handle from {:?} : {:?}", branch, msg)
		};
	}

	async fn connected(socks: &mut Sockets, branch: Branch) {
		console_log!("Connected: {:?}", branch);
		match branch {
			// Branch::Server => socks.server.state = State::Connected(42 as u64),
			Branch::Server => {
				socks.server.state = State::Connected(crate::time_now());
				if socks.right.is_disconnected() && socks.left.is_disconnected() && socks.tmp.is_disconnected() { // add the others
					create_rtc(socks, true).await;
					// TODO webrtc data for the offer
					// let rsp = WebSocketData::OfferSDP("Test".to_string(), None);
					// socks.server.send(Data::WsData(rsp));
				}
			},
			_ => console_log!("Receveing connection from nowhere")
		}
	}

	fn html(socks: &Sockets, id: String, msg: JsValue) {
		match id.as_str() {
			BUTTON_SEND_MESSAGE => {
				let msg = get_input_value(MESSAGE_FIELD_ID);
				set_input_value(MESSAGE_FIELD_ID, "");
				console_log!("need to send {}", msg);
				let rsp = WebSocketData::Message(msg);
				socks.server.send(Data::WsData(rsp));
			}
			_=> console_log!("not handled html element: {}", id)
		}
	}

	fn disconnect(socks: &mut Sockets, cb: &CB, branch: Branch) {
		match branch {
			Branch::Server => reconnect_server(socks, cb),
			_ => console_log!("unsupported disconnect branch: {:?}", branch)
		};
	}
}

pub enum Data {
	WsData(WebSocketData),
	RtcData(String)
}
#[derive(Clone)]
pub enum Socket {
	WebSocket(web_sys::WebSocket),
	WebRTC(RtcPeerConnection)
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
				{ socket.send_with_u8_array(data.into_u8().expect("error while transforming").as_slice()); },
			(Some(Socket::WebRTC(socket)), Data::RtcData(data)) =>
				console_log!("dunno how to send with rtc"),
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