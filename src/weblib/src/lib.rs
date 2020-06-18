use wasm_bindgen::prelude::*;
use futures::channel::mpsc::unbounded;
use futures::future;
use futures::stream::StreamExt;
use js_sys::Date;

// mod webrtc;
mod streams;
use streams::{ Event, Branch };

mod webrtc;

mod cb;
use cb::CB;

mod html;
use html::connect_html;

#[wasm_bindgen]
extern "C" {
	fn alert(s: &str);
	#[wasm_bindgen(js_namespace = console)]
	fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
	// Note that this is using the `log` function imported above during
	// `bare_bones`
	($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

pub fn time_now() -> u64 {
	Date::new_0().get_time() as u64
}

async fn main_loop() {
	let (sender, mut receiver) = unbounded::<Event>();

	let cb = CB::init(sender.clone());
	connect_html(sender.clone());
	sender.unbounded_send(Event::Disconnect(Branch::Server));
	let mut socks = streams::Sockets::default();
	/*
	for_each not working with async block inside we got:receiver
	"A lifetime cannot be determined in the given situation."
	because of `&mut socks`
	
	receiver.for_each(|e| {
		e.execute(sender.clone(), &mut socks, &cb)
		// future::ready(())
	}).await;
	
	So we use the while loop
	*/
	while let Some(e) = receiver.next().await {
		e.execute(sender.clone(), &mut socks, &cb).await;
	}
	console_log!("This should not be reachable")
}

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
	main_loop().await;
	// connect_html(&sockets, &document);
	// sockets.main_loop().await;
	// impl on disconnect to reconnect to the server
	Ok(())
}