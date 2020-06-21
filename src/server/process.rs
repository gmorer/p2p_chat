use protocols::WebSocketData;
use std::net::SocketAddr;
use std::collections::HashMap;
use tungstenite::Message;
use rand;
use futures::channel::mpsc::UnboundedSender;
use crate::PeerMap;

/* WebSocketData to Message
let rsp = match rsp.into_u8() {
	Ok(rsp) => Message::binary(rsp),
	Err(e) => {
		eprintln!("Error while creating data from msg: {}", e);
		return future::err(Error::Protocol(std::borrow::Cow::Borrowed("Internal Error")));
	}
};
*/

type Tx = UnboundedSender<Message>;
type PeerMapLock = HashMap<SocketAddr, Tx>;

fn broadcast_msg(msg: WebSocketData, addr: SocketAddr, peers: &PeerMap) -> Option<WebSocketData> {
	let peers = peers.lock().unwrap();
	let broadcast_recipients = peers
		.iter()
		.filter(|(peer_addr, _)| peer_addr != &&addr)
		.map(|(_, ws_sink)| ws_sink);
	
	match msg.into_u8() {
		Ok(resp) => {
			let resp = Message::Binary(resp);
			for recp in broadcast_recipients {
				recp.unbounded_send(resp.clone()).unwrap();
			};
		}
		Err(e) => eprintln!("error while !parsing message {}", e)
	};
	None
}

fn get_random_peer<'a>(paddr: SocketAddr, peers: &'a PeerMapLock) -> Option<&'a UnboundedSender<Message>> {
	let mut iter = peers.iter();
	let len = peers.len();
	
	// Not sure about that
	for _ in 0..50 {
		let rand = (rand::random::<usize>() + 1) % len;
		if let Some(( addr, sender )) = iter.nth(rand) {
			if &paddr != addr {
				return Some(sender)
			}
		}
	}
	eprintln!("Failed looking for random client");
	None
}

fn offer_sdp(addr: SocketAddr, paddr: Option<SocketAddr>, data: String, peers: &PeerMap) -> Option<WebSocketData> {
	let peers = peers.lock().unwrap();

	let len = peers.len();
	if len < 2 { return None };

	let psender = match paddr {
		Some(paddr) => peers.get(&paddr)?,
		None => get_random_peer(addr, &*peers)?
	};

	let rsp = WebSocketData::OfferSDP(data, Some(addr));
	match rsp.into_u8() {
		Ok(rsp) => { psender.unbounded_send(Message::binary(rsp)); },
		Err(e) => eprintln!("Error while creating data from msg: {}", e)
	};
	None
}

// function for both answerSDP and IceCandidate proxiing
fn proxy(paddr: SocketAddr, msg: WebSocketData, peers: &PeerMap) -> Option<WebSocketData> {
	let peers = peers.lock().unwrap();
	let psender = peers.get(&paddr)?;

	match msg.into_u8() {
		Ok(rsp) => { psender.unbounded_send(Message::Binary(rsp)); },
		Err(e) => eprintln!("Error while creating data from msg: {}", e)
	};
	None
}

pub fn process(addr: SocketAddr, msg: WebSocketData, peers: &PeerMap) -> Option<WebSocketData> {
	match msg {
		WebSocketData::OfferSDP(data, paddr) => offer_sdp(addr , paddr, data, peers),
		WebSocketData::AnswerSDP(data, paddr) => proxy(paddr, WebSocketData::AnswerSDP(data, addr), peers),
		WebSocketData::IceCandidate(data, paddr) => proxy(paddr, WebSocketData::IceCandidate(data, addr), peers),
		WebSocketData::Message(_) =>  broadcast_msg(msg, addr, peers)
	}
}