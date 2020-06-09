use wasm_bindgen::prelude::*;
use web_sys::{ WebSocket, window, BinaryType };
use futures::channel::mpsc::{ UnboundedSender };
use wasm_bindgen::JsCast;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::cb::CB;

use crate::{ log, console_log, SOCKS };
use crate::html::{ MESSAGE_FIELD_ID, BUTTON_SEND_MESSAGE, get_input_value, set_input_value  };


// This functino return an error: RuntimeError: unreachable executed
fn get_actual_timestamp() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("Time went backwards")
		.as_secs()
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Branch {
	Server,
	Right,
	DRight,
	Left,
	Dleft
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum State {
	Disconnected(Option<u64>),
	Connected(u64),
	Waiting(u64),
}

fn reconnect_server() {
	SOCKS.with(|f| {
		let mut f = f.borrow_mut();
		// f.server = Some;
		let cb = f.cb.as_ref().expect("Callback not set while program running");
		let socket_url = format!(
			"ws://{}",
			window().expect("Cannot get the window object").location().host().expect("cannot get the url")
		);
		console_log!("window location: {} ", socket_url);
		match WebSocket::new(&socket_url) {
			Ok(ws) => {
				ws.set_binary_type(BinaryType::Arraybuffer);
				ws.set_onopen(Some(cb.connected_from_server.as_ref().unchecked_ref()));
				ws.set_onclose(Some(cb.disconnect_from_server.as_ref().unchecked_ref()));
				ws.set_onmessage(Some(cb.message_from_server.as_ref().unchecked_ref()));
				f.server.socket = Some(ws);
			}
			Err(e) => console_log!("Error while connecting the server socket: {:?}", e)
		};
	})
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
	Verification, // Created once evry xtime
	Disconnect(Branch),
	Connected(Branch),
	Message(Branch, String), // TODO: Message struct
	Html(String, JsValue) // event from html
}

impl Event {
	pub fn execute(self, sender: UnboundedSender<Event>) {
		match self {
			Event::Verification => console_log!("Getting verification"),
			Event::Disconnect(branch) => Event::disconnect(branch),
			Event::Connected(branch) => Event::connected(branch),
			// Event::Connected(branch) => console_log!("getting connection on {:?}", branch),
			Event::Message(branch, msg) => console_log!("Getting a message from {:?} : {}", branch, msg),
			Event::Html(id, msg) => Event::html(id, msg)
		};
	}

	fn connected(branch: Branch) {
		SOCKS.with(|f| {
			let mut f = f.borrow_mut();
			match branch {
				Branch::Server => f.server.state = State::Connected(42 as u64),
				// Branch::Server => f.server.state = State::Connected(get_actual_timestamp()),
				_ => console_log!("Receveing connection from nowhere")
			}
		})
	}

	fn html(id: String, msg: JsValue) {
		match id.as_str() {
			BUTTON_SEND_MESSAGE => {
				let msg = get_input_value(MESSAGE_FIELD_ID);
				set_input_value(MESSAGE_FIELD_ID, "");
				console_log!("need to send {}", msg);
				SOCKS.with(|f| {
					let f = f.borrow();
					f.server.send(msg);
				})
			}
			_=> console_log!("not handled html element: {}", id)
		}
	}

	fn disconnect(branch: Branch) {
		match branch {
			Branch::Server => reconnect_server(),
			_ => console_log!("unsupported disconnect branch: {:?}", branch)
		};
	}
}

pub struct Pstream {
	pub state: State,
	pub socket: Option<WebSocket>, // will be a trait with rtc, rtc or websocket
}

impl Pstream {
	pub fn send(&self, data: String) {
		match self.state {
			State::Disconnected(x) => console_log!("Cannot send, the client is disconnected since {:?}", x),
			State::Connected(_) => { self.socket.as_ref().expect("connect but no socket, wtf").send_with_str(data.as_str()); },
			State::Waiting(x) => console_log!("Cannot send, we are waiting for answer since {}", x)
		};
	}
}

pub struct Sockets {
	pub server: Pstream,
	// pub right: Option<Pstream>,
	// pub dright: Option<Pstream>,
	// pub left: Option<Pstream>,
	// pub dleft: Option<Pstream>
	pub cb: Option<CB>
}

impl Sockets {
	pub fn default() -> Self {
		Sockets {
			server: Pstream { state: State::Disconnected(None), socket: None },
			cb: None
			// server: Some(Pstream::from_ws(server_ws)),
			// right: None,
			// dright: None,
			// left: None,
			// dleft: None
		}
	}
}