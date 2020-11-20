// use std::convert::TryInto;
use serde::{Serialize, Deserialize};
use std::convert::{ TryFrom, TryInto };

#[derive(PartialEq, Debug)]
pub enum Axe {
	Top,
	Left,
	Right
}

const LETTERS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIKKLMNOPQRSTUVWXYZ0123456789-_"; 
const LETTERS_LENGTH: u64 = 64;
const LENGTHS_BITS: u64 = 6;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, Hash)]
pub struct Id(pub u64);

impl Id {
	pub fn new(long: i32, lat: i32) -> Self {
		Id(long as u32 as u64 + ((lat as u32 as u64) << 32 ))
	}

	pub fn get_lat(&self) -> i32 {
		(self.0 >> 32) as i32
	}

	pub fn to_name(&self) -> String {
		let mut num = self.0;

		let mut result = String::new();
		while num != 0 {
			let character = num & (LETTERS_LENGTH - 1);
			num = num >> LENGTHS_BITS;
			assert!(character < LETTERS_LENGTH, "charachter superior of sizeof_letters: {}", character);
			// unwrap is safe with the assert earlier
			let character = usize::try_from(character).unwrap();
			let character = LETTERS.chars().skip(character).next().unwrap();
			result.push(character);
		}
		result
	}

	pub fn from_name(name: &str) -> Self {
		let mut res: u64 = 0;
		let mut decal = 0;
		for c in name.chars() {
			assert!(decal < 64, "from_name: overflow of u64");
			let index: u64 = LETTERS.find(c).expect("Invalid letter").try_into().expect("is that a 128 bits computer?"); // TODO
			res += index << decal;
			decal += LENGTHS_BITS;
		}
		Id(res)
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

	pub fn get_axe(&self, peer: Self) -> Axe {
		let x = peer.get_long() - self.get_long();
		let y = peer.get_lat() - self.get_lat();
		if x == 0 && y >= 0 {
			Axe::Top
		} else if x == 0 {
			Axe::Right
		} else if y > 0 {
			if (x * 4 / 7).abs() < y.abs() {
				Axe::Top
			} else if x > 0 {
				Axe::Right
			} else {
				Axe::Left
			}
		} else if x > 0 {
			Axe::Right
		} else {
			Axe::Left
		}
	}
}


#[cfg(test)]
mod tests {
	use super::{ Id, Axe };

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
			((5, 0), (0, 12), 17),
			((0, 5), (0, -5), 10),
			((20, 10), (-4, 17), 31),
			// TODOS: more tests
		];
		for ((long1, lat1), (long2, lat2), distance) in coords.into_iter() {
			let id1 = Id::new(long1, lat1);
			let id2 = Id::new(long2, lat2);
			assert_eq!(id1.distance(&id2), distance);
		}
	}

	#[test]
	fn axe_test() {
		let coords = vec![
			(0, 0, Axe::Top),
			(100, 0, Axe::Right),
			(0, 100, Axe::Top),
			(-100, 0, Axe::Left),
			(500, 500, Axe::Top),
			(-50, 50, Axe::Top),
			(50, -50, Axe::Right),
			(-50, -50, Axe::Left)
		];
		let base = Id::new(0, 0);
		for (long, lat, axe) in coords.into_iter() {
			assert_eq!(base.get_axe(Id::new(long, lat)), axe);
		}
	}

	#[test]
	fn name() {
		let ids = vec![
			Id(6271115376183670274),
			Id(565615288521199599),
			Id(13002873412946856394),
			Id(1773930644391763429),
			Id(5334177414139328763),
			Id(4189939283775673285),
			Id(6255372476492508914),
			Id(9714029801535817162),
			Id(17991802674248729776),
			Id(10093706009961291260),
		];

		for id in ids {
			assert_eq!(id.0, Id::from_name(id.to_name().as_str()).0);
 		}
	}
}