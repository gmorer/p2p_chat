pub struct Id(u64);

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
}