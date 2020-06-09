use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{ MessageEvent };
use futures::channel::mpsc::UnboundedSender;
// use crate::{ log, console_log };
use crate::streams::{ Event, Branch };

#[allow(dead_code)]
pub struct CB {
	pub message_from_server: wasm_bindgen::closure::Closure<dyn std::ops::FnMut(JsValue)>,
	pub connected_from_server: wasm_bindgen::closure::Closure<dyn std::ops::FnMut()>,
	pub disconnect_from_server: wasm_bindgen::closure::Closure<dyn std::ops::FnMut()>,
}

impl CB {
	pub fn init(sender: UnboundedSender<Event>) -> Self {
		let sender1 = sender.clone();
		let message_from_server = Closure::wrap(Box::new(move |msg :JsValue| {
			let msg = msg.dyn_ref::<MessageEvent>()
				.expect("not a message event")
				.data()
				.as_string()
				.expect("Cannot convert to string");
			sender1.unbounded_send(Event::Message(Branch::Server, msg));
		}) as Box<dyn FnMut(JsValue)>);
		let sender2 = sender.clone();
		let connected_from_server = Closure::wrap(Box::new(move || {
			sender2.unbounded_send(Event::Connected(Branch::Server));
		}) as Box<dyn FnMut()>);
		let sender3 = sender.clone();
		let disconnect_from_server = Closure::wrap(Box::new(move || {
			sender3.unbounded_send(Event::Disconnect(Branch::Server));
		}) as Box<dyn FnMut()>);
		CB {
			message_from_server,
			connected_from_server,
			disconnect_from_server
		}
	}

}