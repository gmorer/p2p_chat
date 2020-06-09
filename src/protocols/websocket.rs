use serde::{Serialize, Deserialize};
use std::net::SocketAddr;
//

#[derive(Serialize, Deserialize, Debug)]
pub enum Type {
	NewConnection,
	OfferSDP,
	AnswerSDP,
	IceCandidate,
	/* ... */
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WebSocketData {
	pub methode: Type,
	pub data: String,
	pub target: Option<SocketAddr>,
	// target addr
}

impl WebSocketData {
	pub fn from_u8(data: Vec<u8>) -> Result<Self, String> {
		bincode::deserialize(&data[..]).map_err(|e| e.to_string())
	}

	pub fn into_u8(&self) -> Result<Vec<u8>, String> {
		bincode::serialize(self).map_err(|e| e.to_string())
	}
}
