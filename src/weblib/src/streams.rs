use wasm_bindgen::prelude::*;
use web_sys::{ window, BinaryType };
use wasm_bindgen::JsCast;
use std::net::SocketAddr;
use protocols::WebSocketData;

use crate::cb::CB;
use crate::{ log, console_log, Sender };
use crate::html::{ MESSAGE_FIELD_ID, BUTTON_SEND_MESSAGE, get_input_value, set_input_value  };
use crate::webrtc::RTCSocket;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Branch {
	Server,
	Right,
	DRight,
	Left,
	Dleft
}

fn reconnect_server(socks: &mut Sockets, cb: &CB) -> Result<(), String> {
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
			Ok(())
		}
		Err(e) => Err(format!("Error while connecting the server socket: {:?}", e))
	}
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
	pub async fn execute(self, _sender: Sender, socks: &mut Sockets, cb: &CB) -> Result<(), String>{
		match self {
			Event::Verification => Err("Getting verification".to_string()),
			Event::Disconnect(branch) => Event::disconnect(socks, cb, branch),
			Event::Connected(branch) => Event::connected(socks, branch).await,
			Event::ServerMessage(branch, msg) => Event::server_msg(socks, msg, branch).await,
			Event::Html(id, msg) => Event::html(socks, id, msg)
		}
	}

	async fn server_msg(socks: &mut Sockets, msg: WebSocketData, branch: Branch) -> Result<(), String> {
		match msg {
			WebSocketData::OfferSDP(sdp, Some(addr)) => {
				if socks.tmp.is_connected() || socks.tmp.is_locked(None) { // rly None ?
					return Err("Icoming SDP but tmp socket already taken and active (should be moved to a non temporary place".to_string());
				}
				if let Some(Socket::WebRTC(socket)) = &socks.tmp.socket {
					socket.offer(&socks.server, &sdp, addr).await.map_err(|e| format!("{:?}", e))?
				} else {
					let socket = RTCSocket::new(&socks.server, false).await.map_err(|e| format!("{:?}", e))?;
					socket.offer(&socks.server, &sdp, addr).await.map_err(|e| format!("{:?}", e))?;
					socks.tmp.socket = Some(Socket::WebRTC(socket))
				}
				socks.tmp.state = State::Locked(addr);
				Ok(())
			},
			WebSocketData::AnswerSDP(sdp, addr) => {
				if socks.tmp.is_locked(None) {
					return Err("The socket is locked".to_string());
				}
				if let Some(Socket::WebRTC(socket)) = &socks.tmp.socket {
					socket.answer(&socks.server, &sdp, addr).await.map_err(|e| format!("{:?}", e))?;
					socks.tmp.state = State::Locked(addr);
					Ok(())
				} else {
					Err("No soclet object".to_string())
				}
			},
			WebSocketData::IceCandidate(candidate, addr) => {
				if !socks.tmp.is_locked(Some(addr)) {
					return Err(format!("The socket should be locked"));
				}
				if let Some(Socket::WebRTC(socket)) = &socks.tmp.socket {
					socket.ice_candidate(&candidate).await.map_err(|e| format!("{:?}", e))
				} else {
					Err("No soclet object".to_string())
				}
				// console_log!("receiveing IceCandidate: {:?} {:?}", candidate, addr);
				// incomming_ice_candidate(socks, &candidate, addr).await;
			},
			_ => Err(format!("Cannot handle from {:?} : {:?}", branch, msg))
		}
	}

	async fn connected(socks: &mut Sockets, branch: Branch) -> Result<(), String> {
		console_log!("Connected: {:?}", branch);
		match branch {
			// Branch::Server => socks.server.state = State::Connected(42 as u64),
			Branch::Server => {
				socks.server.state = State::Connected(crate::time_now());
				if socks.right.is_disconnected() && socks.left.is_disconnected() && socks.tmp.is_disconnected() { // add the others
					match RTCSocket::new(&socks.server, true).await {
						Ok(socket) => { socks.tmp.socket = Some(Socket::WebRTC(socket)); Ok(()) },
						Err(e) => Err(format!("Error while creating socket: {:?}", e))
					}
				} else { Ok(()) }
			},
			_ => Err("Receveing connection from nowhere".to_string())
		}
	}

	fn html(socks: &Sockets, id: String, msg: JsValue) -> Result<(), String> {
		match id.as_str() {
			BUTTON_SEND_MESSAGE => {
				let msg = get_input_value(MESSAGE_FIELD_ID);
				set_input_value(MESSAGE_FIELD_ID, "");
				console_log!("need to send {}", msg);
				let rsp = WebSocketData::Message(msg);
				socks.server.send(Data::WsData(rsp));
				Ok(())
			}
			_=> Err(format!("not handled html element: id={} msg={:?}", id, msg))
		}
	}

	fn disconnect(socks: &mut Sockets, cb: &CB, branch: Branch) -> Result<(), String> {
		match branch {
			Branch::Server => reconnect_server(socks, cb),
			_ => Err(format!("unsupported disconnect branch: {:?}", branch))
		}
	}
}

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