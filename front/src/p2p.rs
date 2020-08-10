use crossplatform::id::{ Id, Axe };
use crate::webrtc::RTCSocket;

enum Content {
	Message(Option<Id>, String) // Private or Broadcast
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
	// Id of his connection maybe
}

struct Network {
	id: Option<Id>,
	top: Option<Peer>,
	left: Option<Peer>,
	right: Option<Peer>,
	conns: Vec<Peer>
}

impl Network {
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
							self.conns.push(std::mem::replace(&mut self.top, peer)?);
						} else {
							self.conns.push(peer)
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
							self.conns.push(std::mem::replace(&mut self.right, peer)?);
						} else {// TODO: Add deconncetion callback with the id
							self.conns.push(peer)
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
							self.conns.push(std::mem::replace(&mut self.left, peer)?);
						} else {
							self.conns.push(peer)
						}
					},
					None => self.left = Some(peer)
				}
			},
		};
		Some(())
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
			if let Some(index) = self.conns.iter().position(|x| x.id == id) {
				// self.conns[index].disconnect();
				self.conns.remove(index);
			}
		}
	}

	/*
	pub fn send(&self, msg: P2pMessage) {
		// TODO: put the message in memory to not send 2 time the same message
	}
	*/
}