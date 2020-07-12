// use std::convert::TryInto;
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone)]
pub struct Id(pub u64);

impl Id {

	pub fn new(long: i32, lat: i32) -> Self {
		Id(long as u32 as u64 + ((lat as u32 as u64) << 32 ))
	}

	pub fn get_lat(&self) -> i32 {
		(self.0 >> 32) as i32
	}

	pub fn get_long(&self) -> i32 {
		(((self.0) << 32) >> 32) as i32
	}

	pub fn distance(&self, id2: &Self) -> u64 {
		println!("lat = {} - {}", self.get_lat(), id2.get_lat());
		let lat = self.get_lat() as i64 - id2.get_lat() as i64;
		let long = self.get_long() as i64 - id2.get_long() as i64;
		(lat.abs() + long.abs()) as u64
	}
}

#[cfg(test)]
mod tests {
	use super::Id;

	#[test]
	fn id_test() {
		let coords = vec![
			(0, 0),
			(-500, 99),
			(99, -500),
			(500, -99),
			(-99, 500),
			(-99, -99),
			(500, 500)
		];
		for (long, lat) in coords.into_iter() {
			let id = Id::new(long, lat);
			assert_eq!(id.get_long(), long);
			assert_eq!(id.get_lat(), lat);
		}
	}
	#[test]
	fn distance_test()
	{
		let coords = vec![
			((0, 0), (0, 0), 0),
			((5, 0), (0, 12), 13),
			((0, 5), (0, -5), 10),
			((20, 10), (-4, 17), 25),
			// TODOS: more tests
		];
		for ((long1, lat1), (long2, lat2), distance) in coords.into_iter() {
			let id1 = Id::new(long1, lat1);
			let id2 = Id::new(long2, lat2);
			assert_eq!(id1.distance(&id2), distance);
		}
	}
}