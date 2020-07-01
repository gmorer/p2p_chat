use wasm_bindgen::prelude::*;
use web_sys::{ RtcDataChannel };
use crossplatform::proto::WebSocketData;

use crate::{ log, console_log, Sender };
use crate::html::{ MESSAGE_FIELD_ID, BUTTON_SEND_MESSAGE, Html };
use crate::webrtc::RTCSocket;
use crate::websocket::WebSocket;
use crate::streams::{ Sockets, Socket, State, Pstream, Data };

#[derive(Debug)]
#[allow(dead_code)]
pub enum Branch {
	Server,
	Right,
	DRight,
	Left,
	Dleft
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
	Verification, // Created once evry xtime
	Disconnect(Branch),
	Connected(Branch),
	ServerMessage(Branch, WebSocketData), // TODO: Message struct
	Html(String, JsValue), // event from html
	DCObj(RtcDataChannel),
	RtcMessage(String),
	RtcState(bool) // TODO::add state and id
}

impl Event {
	pub async fn execute(self, sender: Sender, socks: &mut Sockets, html: &Html) -> Result<(), String>{
		match self {
			Event::Verification => Err("Getting verification".to_string()),
			Event::Disconnect(branch) => Event::disconnect(socks, branch, html, sender),
			Event::Connected(branch) => Event::connected(socks, branch, sender, html).await,
			Event::ServerMessage(branch, msg) => Event::server_msg(socks, sender, msg, branch, html).await,
			Event::Html(id, msg) => Event::html(socks, id, msg, html),
			Event::DCObj(dc) => Event::dcobj(socks, dc, sender),
			Event::RtcMessage(msg) => Event::rtc_message(msg, html),
			Event::RtcState(state) => Event::rtc_state(socks, state, html),
		}
	}

	fn rtc_message(msg: String, html: &Html) -> Result<(), String> {
		html.chat_msg("Peer", msg.as_str());
		Ok(())
	}

	fn rtc_state(socks: &mut Sockets, state: bool, html: &Html) -> Result<(), String> {
		match state {
			true => html.chat_info("Connection openned with Peer"),
			false => {
				html.chat_info("Connection closed with Peer");
				if let Some(Socket::WebRTC(socket)) = &mut socks.tmp.socket {
					socket.delete();
				}
				socks.tmp = Pstream { state: State::Disconnected(None), socket: None};
			}
		};
		Ok(())

	}

	fn dcobj(socks: &mut Sockets, dc: RtcDataChannel, sender: Sender) -> Result<(), String> {
		match (socks.tmp.state, &mut socks.tmp.socket) {
			(State::Locked(_), Some(Socket::WebRTC(socket)))
			=> socket.set_dc(dc, sender).map_err(|e| format!("Error while setting dc: {:?}", e)),
			_ => Err("Receiving dc obj but tmp isnt locked with an addr".to_string())
		}
	}

	async fn server_msg(socks: &mut Sockets, sender: Sender, msg: WebSocketData, branch: Branch, html: &Html) -> Result<(), String> {
		match msg {
			WebSocketData::OfferSDP(sdp, Some(addr)) => {
				if socks.tmp.is_connected() || socks.tmp.is_locked(None) { // rly None ?
					return Err("Icoming SDP but tmp socket already taken and active (should be moved to a non temporary place".to_string());
				}
				if let Some(Socket::WebRTC(socket)) = &mut socks.tmp.socket {
					socket.offer(&socks.server, &sdp, addr, sender).await.map_err(|e| format!("{:?}", e))?
				} else {
					let mut socket = RTCSocket::new(&socks.server, sender.clone(), html, false).await.map_err(|e| format!("{:?}", e))?;
					socket.offer(&socks.server, &sdp, addr, sender).await.map_err(|e| format!("{:?}", e))?;
					socks.tmp.socket = Some(Socket::WebRTC(socket))
				}
				socks.tmp.state = State::Locked(addr);
				Ok(())
			},
			WebSocketData::AnswerSDP(sdp, addr) => {
				if socks.tmp.is_locked(None) {
					return Err("The socket is locked".to_string());
				}
				if let Some(Socket::WebRTC(socket)) = &mut socks.tmp.socket {
					socket.answer(&socks.server, &sdp, addr, sender).await.map_err(|e| format!("{:?}", e))?;
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
				if let Some(Socket::WebRTC(socket)) = &mut socks.tmp.socket {
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

	async fn connected(socks: &mut Sockets, branch: Branch, sender: Sender, html: &Html) -> Result<(), String> {
		// console_log!("Connected: {:?}", branch);
		match branch {
			// Branch::Server => socks.server.state = State::Connected(42 as u64),
			Branch::Server => {
				html.chat_info("Connected to the server!");
				socks.server.state = State::Connected(crate::time_now());
				if socks.right.is_disconnected() && socks.left.is_disconnected() && socks.tmp.is_disconnected() { // add the others
					match RTCSocket::new(&socks.server, sender, html, true).await {
						Ok(socket) => { socks.tmp.socket = Some(Socket::WebRTC(socket)); Ok(()) },
						Err(e) => Err(format!("Error while creating socket: {:?}", e))
					}
				} else { Ok(()) }
			},
			_ => Err("Receveing connection from nowhere".to_string())
		}
	}

	fn html(socks: &Sockets, id: String, msg: JsValue, html: &Html) -> Result<(), String> {
		match id.as_str() {
			BUTTON_SEND_MESSAGE => {
				let msg = html.get_input_value(MESSAGE_FIELD_ID);
				html.set_input_value(MESSAGE_FIELD_ID, "");
				// console_log!("need to send {}", msg);
				html.chat_msg("Me", msg.as_str());
				// let rsp = WebSocketData::Message(msg);
				// socks.server.send(Data::WsData(rsp));
				socks.tmp.send(Data::RtcData(msg));
				Ok(())
			}
			_=> Err(format!("not handled html element: id={} msg={:?}", id, msg))
		}
	}

	fn disconnect(socks: &mut Sockets, branch: Branch, html: &Html, sender: Sender) -> Result<(), String> {
		match branch {
			Branch::Server => {
				if let Some(Socket::WebSocket(server)) = &socks.server.socket {
					server.delete();
				}
				let socket = WebSocket::new(sender, html)?;
				socks.server.socket = Some(Socket::WebSocket(socket));
				Ok(())
			}
			_ => Err(format!("unsupported disconnect branch: {:?}", branch))
		}
	}
}
