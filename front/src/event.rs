use wasm_bindgen::prelude::*;
use web_sys::{ RtcDataChannel };
use crossplatform::proto::WebSocketData;
use crossplatform::id::Id;

// use crate::{ log, console_log };
use crate::Sender;
use crate::html::{ ids, Html };
use crate::webrtc::RTCSocket;
use crate::websocket::WebSocket;
use crate::streams::{ Sockets, Socket, State, Pstream, Data };

#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
	Verification, // Created once every xtime
	ServerDisconnect,
	ServerConnected,
	ServerMessage(WebSocketData), // TODO: Message struct
	Html(String, JsValue), // event from html
	DCObj(RtcDataChannel), // RTC Data Channel
	TmpId(String), // Should be Id
	RtcState(bool) // For the connecting tmp rtc connection
	// RtcState(Option<Id>) // For the connecting tmp rtc connection
}

impl Event {
	pub async fn execute(self, sender: Sender, socks: &mut Sockets<'_>, html: &Html) -> Result<(), String>{
		match self {
			Event::Verification => Err("Getting verification".to_string()),
			Event::ServerDisconnect => Event::server_disconnect(socks, html, sender),
			Event::ServerConnected => Event::server_connected(socks, sender, html).await,
			Event::ServerMessage(msg) => Event::server_msg(socks, sender, msg, html).await,
			Event::Html(id, msg) => Event::html(socks, id, msg, html),
			Event::DCObj(dc) => Event::dcobj(socks, dc, sender),
			Event::TmpId(msg) => Event::tmp_id(msg, html),
			Event::RtcState(state) => Event::rtc_state(socks, state, html),
		}
	}

	fn tmp_id(msg: String, html: &Html) -> Result<(), String> {
		let _peer_id = Id::from_name(msg.as_str());
		html.chat_info(format!("Connection openned with {}", msg).as_str());
		html.chat_msg("Peer", msg.as_str());
		// TODO: move tmp to a branch
		Ok(())
	}

	fn rtc_state(socks: &mut Sockets, state: bool, html: &Html) -> Result<(), String> {
		match state {
			true => {
				html.chat_info("Connection openned with Peer");
			}
			false => {
				html.chat_info("Connection closed with Peer");
				if let Some(Socket::WebRTC(socket)) = &mut socks.tmp.socket {
					socket.delete();
					socks.tmp = Pstream { state: State::Disconnected(None), socket: None };
					html.fill(ids::TMP_PEER_ID, "None");
				}
				// TODO: remove 
			}
		};
		Ok(())

	}

	fn dcobj(socks: &mut Sockets, dc: RtcDataChannel, sender: Sender) -> Result<(), String> {
		match (socks.tmp.state, &mut socks.tmp.socket) {
			(State::Locked(_), Some(Socket::WebRTC(socket)))
			=> socket.set_dc(dc, sender, socks.id.unwrap()).map_err(|e| format!("Error while setting dc: {:?}", e)),
			_ => Err("Receiving dc obj but tmp isnt locked with an addr".to_string())
		}
	}

	async fn server_msg(socks: &mut Sockets<'_>, sender: Sender, msg: WebSocketData, html: &Html) -> Result<(), String> {
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
					socks.tmp.socket = Some(Socket::WebRTC(socket));
					html.fill(ids::TMP_PEER_ID, "Connecting...");
				}
				socks.tmp.state = State::Locked(addr);
				Ok(())
			},
			WebSocketData::AnswerSDP(sdp, addr) => {
				if socks.tmp.is_locked(None) {
					Err("The socket is locked".to_string())
				}
				else if let Some(Socket::WebRTC(socket)) = &mut socks.tmp.socket {
					socket.answer(&socks.server, &sdp, addr, sender, socks.id.unwrap()).await.map_err(|e| format!("{:?}", e))?;
					socks.tmp.state = State::Locked(addr);
					Ok(())
				} else {
					Err("No soclet object".to_string())
				}
			},
			WebSocketData::IceCandidate(candidate, addr) => {
				if !socks.tmp.is_locked(Some(addr)) {
					Err("The socket should be locked".to_string())
				}
				else if let Some(Socket::WebRTC(socket)) = &mut socks.tmp.socket {
					socket.ice_candidate(&candidate).await.map_err(|e| format!("{:?}", e))
				} else {
					Err("No soclet object".to_string())
				}
			},

			WebSocketData::Id(Some(id)) => {
				socks.id = Some(id);
				html.fill(ids::ID_FIELD_ID, &id.to_name());
				html.chat_info(&format!("Your id is: {}", id.0));
				Ok(())
			}
			_ => Err(format!("Cannot handle from: {:?}", msg))
		}
	}

	async fn server_connected(socks: &mut Sockets<'_>, sender: Sender, html: &Html) -> Result<(), String> {
		// console_log!("Connected: {:?}", branch);
		html.chat_info("Connected to the server!");
		socks.server.state = State::Connected(crate::time_now());
		// Ask or set the id server side
		socks.server.send(Data::WsData(WebSocketData::Id(socks.id)));
		if socks.tmp.is_disconnected() { // add the others
			match RTCSocket::new(&socks.server, sender, html, true).await {
				Ok(socket) => { socks.tmp.socket = Some(Socket::WebRTC(socket)); Ok(()) },
				Err(e) => Err(format!("Error while creating socket: {:?}", e))
			}
		} else { Ok(()) }
	}

	fn html(socks: &Sockets, id: String, msg: JsValue, html: &Html) -> Result<(), String> {
		match id.as_str() {
			ids::BUTTON_SEND_MESSAGE => {
				let msg = html.get_input_value(ids::MESSAGE_FIELD_ID);
				let msg = msg.trim();
				if msg.is_empty() { return Ok(()) }
				html.set_input_value(ids::MESSAGE_FIELD_ID, "");
				// console_log!("need to send {}", msg);
				html.chat_msg("Me", msg);
				// let rsp = WebSocketData::Message(msg);
				// socks.server.send(Data::WsData(rsp));
				socks.tmp.send(Data::RtcData(msg.to_string()));
				Ok(())
			}
			_=> Err(format!("not handled html element: id={} msg={:?}", id, msg))
		}
	}

	fn server_disconnect(socks: &mut Sockets, html: &Html, sender: Sender) -> Result<(), String> {
		if let Some(Socket::WebSocket(server)) = &socks.server.socket {
			server.delete();
		}
		let socket = WebSocket::new(sender, html)?;
		socks.server.socket = Some(Socket::WebSocket(socket));
		Ok(())
	}
}
