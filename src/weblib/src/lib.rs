use wasm_bindgen::prelude::*;
use futures::channel::mpsc::unbounded;
use futures::future;
use futures::stream::StreamExt;

// mod webrtc;
mod streams;
use streams::{ Event, Branch };

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

async fn main_loop() {
	let (sender, receiver) = unbounded::<Event>();

	let cb = CB::init(sender.clone());
	let mut socks = streams::Sockets::default();
	connect_html(sender.clone());
	sender.unbounded_send(Event::Disconnect(Branch::Server));
	// keep callback in memory to not create memory leaks with forget()
	// SOCKS.with(|f| { f.borrow_mut().cb = Some(CB::init(sender.clone())); });
	receiver.for_each(|e| {
		e.execute(sender.clone(), &mut socks, &cb);
		future::ready(())
	}).await;
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