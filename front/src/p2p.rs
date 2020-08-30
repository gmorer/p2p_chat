use wasm_bindgen::{ JsValue, JsCast };
use wasm_bindgen::closure::Closure;
use js_sys::ArrayBuffer;
use crossplatform::id::{ Id, Axe };
use crossplatform::proto_rtc::{ RTCData, RTCContent };
use crate::html::{ Html, ids };
use crate::webrtc::RTCSocket;
use crate::event::Event;
use crate::{ log, console_log };
use crate::{ Sender };
use web_sys::{
	MessageEvent,
};

#[derive(Debug)]
struct Peer {
	id: Id,
	socket: RTCSocket
	// Id of his connections maybe
}

impl Peer {
	pub fn send(&self, from: Id, data_from: Id, data: &[u8]) {
		if self.id != from && self.id != data_from {
			self.socket.send(data);
		}
	}
}

fn new_cb(id: Id, sender: Sender, socket: &mut RTCSocket)
{
	let sender_cl = sender.clone();
	let onclose_callback = Closure::wrap(Box::new(move |_arg: JsValue| {
		sender_cl.send(Event::RtcState(false));
	}) as Box<dyn FnMut(JsValue)>);
	socket.channel.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
	let onmessage_callback = Closure::wrap(Box::new(move |ev: JsValue| {
		let ev = MessageEvent::from(ev);
		if let Ok(abuf) =  ev.data().dyn_into::<ArrayBuffer>() {
			let array = js_sys::Uint8Array::new(&abuf).to_vec();
			let msg = RTCData::from_u8(array).expect("Invalid incomming");
			sender.send(Event::RTCMessage(id, msg));
		} else {
			console_log!("Invalid: {:?}", ev);
		}
	}) as Box<dyn FnMut(JsValue)>);
	socket.channel.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
	socket.cbs.push(onclose_callback);
	socket.cbs.push(onmessage_callback);
}

#[derive(Debug)]
pub struct Network<'a> {
	pub id: Option<Id>, // Remove Option ?
	top: Option<Peer>,
	left: Option<Peer>,
	right: Option<Peer>,
	peer_cache: Vec<Peer>,
	html: &'a Html
}

impl<'a> Network<'a> {
	pub fn new(html: &'a Html) -> Self {
		Network {
			id: None,
			top: None,
			left: None,
			right: None,
			peer_cache: vec!(),
			html
		}
	}

	pub fn insert(&mut self, mut socket: RTCSocket, id: Id, sender: Sender) -> Option<()> {
		let self_id = self.id?;
		let distance = self_id.distance(&id);
		new_cb(id, sender, &mut socket);
		let peer = Peer { id, socket };
		match self_id.get_axe(id) {
			Axe::Top => {
				match &self.top {
					Some(top) => {
						if self_id.distance(&top.id) > distance {
							let peer = Some(peer);
							self.peer_cache.push(std::mem::replace(&mut self.top, peer)?);
						} else {
							self.peer_cache.push(peer)
						}
					},
					None => self.top = Some(peer)
				}
			},
			Axe::Right => {
				match &self.right {
					Some(right) => {
						if self_id.distance(&right.id) > distance {
							let peer = Some(peer);
							self.peer_cache.push(std::mem::replace(&mut self.right, peer)?);
						} else {
							self.peer_cache.push(peer)
						}
					},
					None => self.right = Some(peer)
				}
			},
			Axe::Left => {
				match &self.left {
					Some(left) => {
						if self_id.distance(&left.id) > distance {
							let peer = Some(peer);
							self.peer_cache.push(std::mem::replace(&mut self.left, peer)?);
						} else {
							self.peer_cache.push(peer)
						}
					},
					None => self.left = Some(peer)
				}
			}
		};
		self.refresh_html();
		Some(())
	}

	pub fn process(&self, data: &RTCData, from: Id) -> Result<(), String> {
		match &data.content {
			RTCContent::Message(msg) => {
				if let Some(target) = data.to {
					if target == self.id.expect("Should have an id") {
						self.html.chat_private(data.from.to_name().as_str(), msg.as_str())
					} else {
						// self.send_to() 
						self.send(data, from);
					}
				} else {
					self.html.chat_msg(data.from.to_name().as_str(), msg.as_str());
					self.send(data, from);
				}
			},
			RTCContent::Received(_id, _timestamp) => { }
			RTCContent::NotFound => { },
		}
		Ok(())
	}

	pub fn refresh_html(&self) {
		self.html.fill(ids::TOP_PEER_ID, self.top.as_ref().and_then(|a| Some(a.id.to_name())).unwrap_or("None".to_string()).as_str());
		self.html.fill(ids::LEFT_PEER_ID, self.left.as_ref().and_then(|a| Some(a.id.to_name())).unwrap_or("None".to_string()).as_str());
		self.html.fill(ids::RIGHT_PEER_ID, self.right.as_ref().and_then(|a| Some(a.id.to_name())).unwrap_or("None".to_string()).as_str());
		self.html.fill(ids::CACHE_PEER_ID, "");
		self.peer_cache.iter().for_each(|peer| {
			self.html.append(ids::CACHE_PEER_ID, format!("<span>{}</span>", peer.id.to_name()).as_str())
		})
	}

	pub fn remove(&mut self, id: Id) -> Result<(), String>{
		// TODO: Replace one of the side from a peer from the cache
		if let Some(top) = &self.top {
			if top.id == id {
				self.top = None
			}
		} else if let Some(right) = &self.right {
			if right.id == id {
				self.right = None
			}
		} else if let Some(left) = &self.left {
			if left.id == id {
				self.left = None
			}
		} else {
			if let Some(index) = self.peer_cache.iter().position(|x| x.id == id) {
				// self.conns[index].disconnect();
				self.peer_cache.remove(index);
			} else {
				return Err("Unknow Peer as disconnected".to_string());
			}
		}
		self.html.chat_info(format!("{} as disconnected.", id.to_name()).as_str());
		self.refresh_html();
		Ok(())
	}

	pub fn send(&self, data: &RTCData, from: Id) {
		// TODO: put the message in memory to not send 2 time the same message
		if let Some(_id) = data.to {
			// Send to specifiq user
		} else {
			// Send to all users
			let data_from = data.from;
			let data = data.into_u8().expect("cannot serialize");
			let data = data.as_slice();
			if let Some(top) = &self.top {
				top.send(from, data_from, data);
			}
			if let Some(right) = &self.right {
				right.send(from, data_from, data);
			}
			if let Some(left) = &self.left {
				left.send(from, data_from, data);
			}
			self.peer_cache.iter().for_each(|peer| {
				peer.send(from, data_from, data);
			})
		}
	}
}