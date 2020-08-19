use crossplatform::id::{ Id, Axe };
use crate::webrtc::RTCSocket;
use crate::html::{ Html, ids };

pub enum Content {
	Message(Id, String), // Private or Broadcast
	// RTC Stuff
	// Mapping Stuff
}

struct P2pMessage {
	timestamp: u32,
	from: Id,
	content: Content
}

struct Peer {
	id: Id,
	socket: RTCSocket
	// Id of his connections maybe
}

pub struct Network<'a> {
	id: Option<Id>, // Remove Option ?
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

	pub fn insert(&mut self, socket: RTCSocket, id: Id) -> Option<()> {
		// TODO: change callbacks with the id inside
		let distance = self.id?.distance(&id);
		let peer = Peer { id, socket };
		match self.id?.get_axe(id) {
			Axe::Top => {
				match &self.top {
					Some(top) => {
						if self.id?.distance(&top.id) > distance {
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
						if self.id?.distance(&right.id) > distance {
							let peer = Some(peer);
							self.peer_cache.push(std::mem::replace(&mut self.right, peer)?);
						} else {// TODO: Add deconncetion callback with the id
							self.peer_cache.push(peer)
						}
					},
					None => self.right = Some(peer)
				}
			},
			Axe::Left => {
				match &self.left {
					Some(left) => {
						if self.id?.distance(&left.id) > distance {
							let peer = Some(peer);
							self.peer_cache.push(std::mem::replace(&mut self.left, peer)?);
						} else {
							self.peer_cache.push(peer)
						}
					},
					None => self.left = Some(peer)
				}
			},
		};
		Some(())
	}

	// TODO: store html struct or a ref
	pub fn refresh_html(&self) {
		self.html.fill(ids::TOP_PEER_ID, self.top.as_ref().and_then(|a| Some(a.id.to_name())).unwrap_or("None".to_string()).as_str());
		self.html.fill(ids::LEFT_PEER_ID, self.left.as_ref().and_then(|a| Some(a.id.to_name())).unwrap_or("None".to_string()).as_str());
		self.html.fill(ids::RIGHT_PEER_ID, self.right.as_ref().and_then(|a| Some(a.id.to_name())).unwrap_or("None".to_string()).as_str());
		self.html.fill(ids::CACHE_PEER_ID, "");
		self.peer_cache.iter().for_each(|peer| {
			self.html.append(ids::CACHE_PEER_ID, format!("<span>{}</span>", peer.id.to_name()).as_str())
		})
	}

	pub fn remove(&mut self, id: Id) {
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
			}
		}
	}

	/*
	pub fn send(&self, msg: P2pMessage) {
		// TODO: put the message in memory to not send 2 time the same message
	}
	*/
}