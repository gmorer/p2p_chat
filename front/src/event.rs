use wasm_bindgen::prelude::*;
use web_sys::{ RtcDataChannel };
use crossplatform::proto_ws::WebSocketData;
use crossplatform::proto_rtc::{ RTCData, RTCContent };
use crossplatform::id::Id;

// use crate::{ log, console_log };
use crate::Sender;
use crate::html::{ ids, Html };
use crate::webrtc::RTCSocket;
use crate::websocket::WebSocket;
use crate::streams::{ Sockets, Socket, State, Pstream, Data };
use crate::p2p::Network;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
	ServerDisconnect,
	ServerConnected,
	ServerMessage(WebSocketData), // TODO: Message struct
	Html(String, JsValue), // event from html
	DCObj(RtcDataChannel), // RTC Data Channel
	TmpId(String), // Should be Id
	RtcState(bool), // Become RTCDisconnect with Option<Id> tmp if none
	RTCMessage(Id, RTCData),
	RTCDisconnect(Id)
	// RTCMessage
}

impl Event {
	pub async fn execute<'a>(self, sender: Sender, socks: &mut Sockets<'a>, html: &'a Html) -> Result<(), String>{
		match self {
			// Server Event
			Event::ServerDisconnect => Event::server_disconnect(socks, html, sender),
			Event::ServerConnected => Event::server_connected(socks, sender, html).await,
			Event::ServerMessage(msg) => Event::server_msg(socks, sender, msg, html).await,

			// RTC Events
			Event::DCObj(dc) => Event::dcobj(socks, dc, sender),
			Event::TmpId(msg) => Event::tmp_id(socks, msg, html, sender),
			Event::RtcState(state) => Event::rtc_state(socks, state, html),
			Event::RTCMessage(id, data) => socks.network.as_ref().ok_or("Should have a network")?.process(&data, id),
			Event::RTCDisconnect(id) => socks.network.as_mut().ok_or("Should have a network")?.remove(id),
			// Html Event
			Event::Html(id, msg) => Event::html(socks, id, msg, html),
			// data => Err(format!("cannot handle {:?}", data))
		}
	}

	fn tmp_id(socks: &mut Sockets, msg: String, html: &Html, sender: Sender) -> Result<(), String> {
		let network = socks.network.as_mut().ok_or("You are not connected to the network")?;
		let socket = std::mem::replace(&mut socks.tmp.socket, None);
		html.fill(ids::TMP_PEER_ID, "None");
		socks.tmp.state = State::Disconnected(None);
		// socks.tmp.
		if let Some(Socket::WebRTC(socket)) = socket {
			let peer_id = Id::from_name(msg.as_str());
			network.insert(socket.clone(), peer_id, sender);
			// TODO: delete tmp (state... , not cbs)

			html.chat_info(format!("Connection openned with {}", msg).as_str());
			html.chat_msg("Peer", msg.as_str());
			Ok(())
		} else {
			Err("Invlaid type for tmp socket".to_string())
		}
	}

	// True version is a duplicat fo tmp_i
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
		let id = socks.network.as_ref().ok_or("Should have a network")?.id;
		match (socks.tmp.state, &mut socks.tmp.socket) {
			(State::Locked(_), Some(Socket::WebRTC(socket)))
			=> socket.set_dc(dc, sender, id).map_err(|e| format!("Error while setting dc: {:?}", e)),
			_ => Err("Receiving dc obj but tmp isnt locked with an addr".to_string())
		}
	}

	async fn server_msg<'a>(socks: &mut Sockets<'a>, sender: Sender, msg: WebSocketData, html: &'a Html) -> Result<(), String> {
		match msg {
			WebSocketData::OfferSDP(sdp, Some(addr)) => {
				if socks.tmp.is_connected() || socks.tmp.is_locked(None) { // rly None ?
					return Err("Icoming SDP but tmp socket already taken and active (should be moved to a non temporary place".to_string());
				}
				if let Some(Socket::WebRTC(socket)) = &mut socks.tmp.socket {
					socket.offer(&socks.server, &sdp, addr, sender).await.map_err(|e| format!("{:?}", e))?;
					html.fill(ids::TMP_PEER_ID, "Connecting...");
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
				let id = socks.network.as_ref().ok_or("Should have a network")?.id;
				if socks.tmp.is_locked(None) {
					Err("The socket is locked".to_string())
				}
				else if let Some(Socket::WebRTC(socket)) = &mut socks.tmp.socket {
					socket.answer(&socks.server, &sdp, addr, sender, id).await.map_err(|e| format!("{:?}", e))?;
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
				if socks.network.is_none() {
					socks.network = Some(Network::new(html, id));
					html.fill(ids::ID_FIELD_ID, &id.to_name());
					html.chat_info(&format!("Your id is: {}", id.0));
				}
				Ok(())
			}
			_ => Err(format!("Cannot handle from: {:?}", msg))
		}
	}

	async fn server_connected(socks: &mut Sockets<'_>, sender: Sender, html: &Html) -> Result<(), String> {
		html.chat_info("Connected to the server!");
		socks.server.state = State::Connected(crate::time_now());
		// Ask or set the id server side
		socks.server.send(Data::WsData(WebSocketData::Id(socks.network.as_ref().and_then(|net| Some(net.id)))));
		if socks.tmp.is_disconnected() { // add the others
			match RTCSocket::new(&socks.server, sender, html, true).await {
				Ok(socket) => { socks.tmp.socket = Some(Socket::WebRTC(socket)); Ok(()) },
				Err(e) => Err(format!("Error while creating socket: {:?}", e))
			}
		} else { Ok(()) }
	}

	fn html(socks: &Sockets, id: String, msg: JsValue, html: &Html) -> Result<(), String> {
		let network = socks.network.as_ref().ok_or("You are not connected to the network")?;
		match id.as_str() {
			ids::BUTTON_SEND_MESSAGE => {
				let msg = html.get_input_value(ids::MESSAGE_FIELD_ID);
				let msg = msg.trim();
				if msg.is_empty() { return Ok(()) }
				html.set_input_value(ids::MESSAGE_FIELD_ID, "");
				html.chat_msg("Me", msg);
				let msg = RTCData {
					to: None,
					id: 0,
					timestamp: 0,
					from: network.id,
					content: RTCContent::Message(msg.to_string())
				};
				network.send(&msg, network.id);
				// let rsp = WebSocketData::Message(msg);
				// socks.server.send(Data::WsData(rsp));
				// socks.tmp.send(Data::RtcData(msg.to_string()));

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
