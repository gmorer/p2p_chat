// Structure that well be send across the websocket
// in a client-server connection

use serde::{Serialize, Deserialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug)]
pub struct IceCandidateStruct {
	pub candidate: String,
	pub sdp_mid: Option<String>,
	pub sdp_m_line_index: Option<u16>
}

// Make it an enum ? (no method field)
#[derive(Serialize, Deserialize, Debug)]
pub enum WebSocketData {
	OfferSDP(String, Option<SocketAddr>),
	AnswerSDP(String, SocketAddr),
	IceCandidate(IceCandidateStruct, SocketAddr),
	Message(String), // For testing purpose
	// TODO: whoami
}

impl WebSocketData {
	pub fn from_u8(data: Vec<u8>) -> Result<Self, String> {
		bincode::deserialize(&data[..]).map_err(|e| e.to_string())
	}

	pub fn into_u8(&self) -> Result<Vec<u8>, String> {
		bincode::serialize(self).map_err(|e| e.to_string())
	}
}
