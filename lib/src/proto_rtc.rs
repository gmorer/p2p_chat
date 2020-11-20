use serde::{Serialize, Deserialize};
use crate::id::Id;

// Present here for the serde crate
#[derive(Serialize, Deserialize, Debug, Hash)]
pub struct RTCData {
	pub id: u32, // Random generated id
	pub timestamp: u32,
	pub from: Id,
	pub content: RTCContent,
	pub to: Option<Id> // target or broadcast
}

#[derive(Serialize, Deserialize, Debug, Hash)]
pub enum RTCContent {
	Message(String), // Private or Broadcast
	Received(u32, u32), // id and timestamp
	NotFound, // Nearest peer doesnt know
}

impl RTCData {
	pub fn from_u8(data: Vec<u8>) -> Result<Self, String> {
		bincode::deserialize(&data[..]).map_err(|e| e.to_string())
	}

	pub fn into_u8(&self) -> Result<Vec<u8>, String> {
		bincode::serialize(self).map_err(|e| e.to_string())
	}
}