use web_sys::BinaryType;
use wasm_bindgen::{ JsValue, JsCast };
use wasm_bindgen::closure::Closure;
use js_sys::Uint8Array;
use crossplatform::proto::WebSocketData;
use web_sys::{ MessageEvent };
use crate::html::Html;
use crate::{ log, console_log, Sender };
use crate::event::{ Event, Branch };


pub struct WebSocket {
	socket: web_sys::WebSocket,
	cbs: Vec<Closure<dyn FnMut (JsValue)>> // keep the callbacks in memory
}

impl Clone for WebSocket {
	fn clone(&self) -> Self {
		Self {
			socket: self.socket.clone(),
			cbs: vec!() // callbacks need to be in online one place
		}
	}
}

impl WebSocket {
	pub fn new(sender: Sender, html: &Html) -> Result<Self, String> {
		let socket_url = format!(
			"ws://{}",
			html.window.location().host().expect("cannot get the url")
		);
		html.chat_info("Reconnecting to the server...");
		let socket = match web_sys::WebSocket::new(&socket_url) {
			Ok(socket) => socket,
			Err(e) => return Err(format!("Error while connecting the server socket: {:?}", e))
		};
		socket.set_binary_type(BinaryType::Arraybuffer);
		let mut cbs = vec!();

		/* Initalize the cbs */
		let sender1 = sender.clone();
		let message_from_server = Closure::wrap(Box::new(move |msg :JsValue| {
			let msg = msg.dyn_ref::<MessageEvent>()
				.expect("not a message event")
				.data();
			let msg = Uint8Array::new(&msg).to_vec();
			let msg = WebSocketData::from_u8(msg).expect("Cannot convert server response");
			sender1.send(Event::ServerMessage(Branch::Server, msg));
		}) as Box<dyn FnMut(JsValue)>);
		let sender2 = sender.clone();
		let connected_from_server = Closure::wrap(Box::new(move |_args: JsValue| {
			sender2.send(Event::Connected(Branch::Server));
		}) as Box<dyn FnMut(JsValue)>);
		let sender3 = sender.clone();
		let disconnect_from_server = Closure::wrap(Box::new(move |_args: JsValue| {
			sender3.send(Event::Disconnect(Branch::Server));
		}) as Box<dyn FnMut(JsValue)>);

		/* Set the cbs */
		socket.set_onmessage(Some(message_from_server.as_ref().unchecked_ref()));
		cbs.push(message_from_server);
		socket.set_onopen(Some(connected_from_server.as_ref().unchecked_ref()));
		cbs.push(connected_from_server);
		socket.set_onclose(Some(disconnect_from_server.as_ref().unchecked_ref()));
		cbs.push(disconnect_from_server);
		Ok(Self {
			socket,
			cbs
		})
	}
	
	pub fn send(&self, data: &WebSocketData) {
		if let Err(e) = self.socket.send_with_u8_array(data.into_u8().expect("error while transforming").as_slice()) {
			console_log!("Error while sending to server: {:?}", e)
		}
	}

	pub fn delete(&self) {
		self.socket.close();
	}
}