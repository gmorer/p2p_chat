// use protocols::WebSocketData;
use crossplatform::proto::WebSocketData;
use std::net::SocketAddr;
use std::collections::HashMap;
use tungstenite::Message;
use futures::channel::mpsc::UnboundedSender;
use crate::PeerMap;
use crate::Id;

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
type PeerMapLock = HashMap<SocketAddr, (Id, Tx)>;

fn broadcast_msg(msg: WebSocketData, addr: SocketAddr, peers: &PeerMap) -> Option<WebSocketData> {
	let peers = peers.lock().unwrap();
	let broadcast_recipients = peers
		.iter()
		.filter(|(peer_addr, _)| peer_addr != &&addr)
		.map(|(_, (_id, ws_sink))| ws_sink);
	
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

fn closest_peer<'a>(addr: SocketAddr, peers: &'a PeerMapLock) -> Option<&'a UnboundedSender<Message>> {
	let (id, _) = peers.get(&addr)?;

	println!("Peers: {:?}", peers);
	let mut distance = u64::MAX;
	let mut res = None;

	for (_, (i_id, i_sender)) in peers.iter() {
		if id == i_id {
			continue;
		}
		let i_distance = id.distance(i_id);
		if i_distance < distance {
			distance = i_distance;
			res = Some(i_sender);
		}
	}
	res
}

fn offer_sdp(addr: SocketAddr, paddr: Option<SocketAddr>, data: String, peers: &PeerMap) -> Option<WebSocketData> {
	let peers = peers.lock().unwrap();

	let len = peers.len();
	if len < 2 { return None };

	let psender = match paddr {
		Some(paddr) => &peers.get(&paddr)?.1,
		None => closest_peer(addr, &*peers)?
	};

	println!("got a psender");
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
	let (_id, psender) = peers.get(&paddr)?;

	match msg.into_u8() {
		Ok(rsp) => { psender.unbounded_send(Message::Binary(rsp)); },
		Err(e) => eprintln!("Error while creating data from msg: {}", e)
	};
	None
}

fn send_id(addr: SocketAddr, peers: &PeerMap) -> Option<WebSocketData> {
	let peers = peers.lock().unwrap();
	let (id, _sender) = peers.get(&addr)?;
	Some(WebSocketData::Id(Some(id.clone())))
}

pub fn process(addr: SocketAddr, msg: WebSocketData, peers: &PeerMap) -> Option<WebSocketData> {
	match msg {
		WebSocketData::OfferSDP(data, paddr) => offer_sdp(addr , paddr, data, peers),
		WebSocketData::AnswerSDP(data, paddr) => proxy(paddr, WebSocketData::AnswerSDP(data, addr), peers),
		WebSocketData::IceCandidate(data, paddr) => proxy(paddr, WebSocketData::IceCandidate(data, addr), peers),
		WebSocketData::Message(_) =>  broadcast_msg(msg, addr, peers),
		WebSocketData::Id(_) => send_id(addr, peers)
	}
}