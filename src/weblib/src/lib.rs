use wasm_bindgen::prelude::*;
use web_sys::{ window, Document };
use futures::channel::mpsc::unbounded;
use futures::future;
use futures::stream::StreamExt;
use std::cell::RefCell;

// mod webrtc;
mod streams;
use streams::{ Event, Branch };

mod cb;
use cb::CB;

mod html;
use html::connect_html;

// TODO: Better way to get global for closure
thread_local! {
	pub static SOCKS: RefCell<streams::Sockets> = RefCell::new(streams::Sockets::default());
}

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

async fn main_loop(document: &Document) {
	let (sender, receiver) = unbounded::<Event>();

	// keep callback in memory to not create memory leaks with forget()
	SOCKS.with(|f| { f.borrow_mut().cb = Some(CB::init(sender.clone())); });
	connect_html(document, sender.clone());
	sender.unbounded_send(Event::Disconnect(Branch::Server));
	receiver.for_each(|e| {
		e.execute(sender.clone());
		future::ready(())
	}).await;
	console_log!("This should not be reachable")
}

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
	let window = window().expect("Cannot get the window object");
	let document = window.document().expect("window should have a document");
	main_loop(&document).await;
	// connect_html(&sockets, &document);
	// sockets.main_loop().await;
	// impl on disconnect to reconnect to the server
	Ok(())
}