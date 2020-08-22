use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{ Document, HtmlElement, window, Window, HtmlInputElement, Element };
use crate::event::{ Event };
use crate::Sender;

// Html element ids
pub mod ids {
	pub const BUTTON_SEND_MESSAGE: &'static str = "send_message";
	pub const MESSAGE_FIELD_ID: &'static str = "message_field";
	pub const MESSAGE_BOX_ID: &'static str = "message_box";
	pub const ID_FIELD_ID: &'static str = "id_field";
	pub const TMP_PEER_ID: &'static str = "tmp_peer";
	pub const TOP_PEER_ID: &'static str = "top_peer";
	pub const LEFT_PEER_ID: &'static str = "left_peer";
	pub const RIGHT_PEER_ID: &'static str = "right_peer";
	pub const CACHE_PEER_ID: &'static str = "cache_peer";
}

// TODO: global input hashmap (gota go fast)

// Setup a on click event, the id and the msg will be send to the stream
fn on_click(elem: &Element, id: &'static str, sender: Sender) {
	let open_handler =  Closure::wrap(Box::new(move |msg| {
		sender.send(Event::Html(id.to_string(), msg));
	}) as Box<dyn FnMut(JsValue)>);
	elem.dyn_ref::<HtmlElement>()
		.expect("button is not an html element")
		.set_onclick(Some(open_handler.as_ref().unchecked_ref()));
	open_handler.forget();
}

#[derive(Debug)]
pub struct Html {
	pub elements: HashMap<String, Element>,
	pub window: Window,
	pub document: Document
}

impl Html {
	pub fn new(sender: Sender) -> Self {
		let mut elements = HashMap::new();
		let window = window().expect("Cannot get the window object");
		let document = window.document().expect("window should have a document");
		let ids = [
			(ids::MESSAGE_FIELD_ID, false),
			(ids::MESSAGE_BOX_ID, false),
			(ids::BUTTON_SEND_MESSAGE, true),
			(ids::ID_FIELD_ID, false),
			(ids::TMP_PEER_ID, false),
			(ids::TOP_PEER_ID, false),
			(ids::LEFT_PEER_ID, false),
			(ids::RIGHT_PEER_ID, false),
			(ids::CACHE_PEER_ID, false)
		];
		for (id, click) in ids.iter() {
			if let Some(element) = document.get_element_by_id(id) {
				if *click {
					on_click(&element, id, sender.clone());
				}
				elements.insert(id.to_string(), element);
			}
		}

		let ret = Self {
			elements,
			window,
			document
		};
		ret
	}

	fn chat_bottom_scroll(&self) {
		if let Some(elem) = self.elements.get(&ids::MESSAGE_BOX_ID.to_string()) {
			elem.set_scroll_top(elem.scroll_height());
		}
	}

	pub fn get_input_value(&self, id: &str) -> String {
		if let Some(elem) = self.elements.get(&id.to_string()) {
			let input = elem
				.dyn_ref::<HtmlInputElement>()
				.expect("Input is not an input");
			input.value()
		} else { "".to_string() }
	}
	
	pub fn set_input_value(&self, id: &str, value: &str) {
		if let Some(elem) = self.elements.get(&id.to_string()) {
			let input = elem
				.dyn_ref::<HtmlInputElement>()
				.expect("Input is not an input");
			input.set_value(value);
		}
	}

	pub fn append(&self, id: &str, value: &str) {
		if let Some(elem) = self.elements.get(&id.to_string()) {
			elem.insert_adjacent_html("beforeend", value).unwrap_or(());
		}
	}

	pub fn fill(&self, id: &str, value: &str) {
		if let Some(elem) = self.elements.get(&id.to_string()) {
			elem.set_inner_html(value);
		}
	}

	pub fn chat_msg(&self, user: &str, msg: &str) {
		self.append(ids::MESSAGE_BOX_ID, format!("<p><b>{}: </b> {}</p>", user, msg).as_str());
		self.chat_bottom_scroll();
	}

	pub fn chat_info(&self, msg: &str) {
		self.append(ids::MESSAGE_BOX_ID, format!("<p><i>{}</i></p>", msg).as_str());
		self.chat_bottom_scroll();
	}

	pub fn chat_error(&self, msg: &str) {
		self.append(ids::MESSAGE_BOX_ID, format!("<p><b style=\"color: red\" >{}</b></p>", msg).as_str());
		self.chat_bottom_scroll();
	}

	pub fn chat_private(&self, user: &str, msg: &str) {
		self.append(ids::MESSAGE_BOX_ID, format!("<p><i><b>{}: </b> {}</i></p>", user, msg).as_str());
		self.chat_bottom_scroll();
	}
}