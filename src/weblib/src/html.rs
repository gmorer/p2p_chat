use futures::channel::mpsc::UnboundedSender;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{ Document, HtmlElement, window, HtmlInputElement };
use crate::streams::{ Event };


pub const BUTTON_SEND_MESSAGE: &'static str = "send_message";
pub const MESSAGE_FIELD_ID: &'static str = "message_field";

// TODO: global input hashmap (gota go fast)
pub fn get_input_value(id: &str) -> String {
	let input = window()
		.expect("Cannot get the window")
		.document()
		.expect("Cannot get the document")
		.get_element_by_id(id)
		.expect("cannot find the message input");
	let input = input
		.dyn_ref::<HtmlInputElement>()
		.expect("Input is not an input");
	input.value()
}

pub fn set_input_value(id: &str, value: &str) {
	let input = window()
		.expect("Cannot get the window")
		.document()
		.expect("Cannot get the document")
		.get_element_by_id(id)
		.expect("cannot find the message input");
		let input = input
		.dyn_ref::<HtmlInputElement>()
		.expect("Input is not an input");
	input.set_value("");
}

// Setup a on click event, the id and the msg will be send to the stream
fn on_click(document: &Document, id: &'static str, sender: UnboundedSender<Event>) {
	let open_handler =  Closure::wrap(Box::new(move |msg| {
		sender.unbounded_send(Event::Html(id.to_string(), msg));
	}) as Box<dyn FnMut(JsValue)>);
	document
		.get_element_by_id(id)
		.expect("cannot fidn the send message button")
		.dyn_ref::<HtmlElement>()
		.expect("button is not an html element")
		.set_onclick(Some(open_handler.as_ref().unchecked_ref()));
	// This function is called only once so it's ok
	open_handler.forget();

}

pub fn connect_html(document: &Document, sender: UnboundedSender<Event>) {
	let sender1 = sender.clone();
	on_click(document, BUTTON_SEND_MESSAGE, sender);
}
