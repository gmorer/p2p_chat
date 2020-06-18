use std::net::SocketAddr;
use wasm_bindgen::{ JsValue, JsCast };
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use js_sys::{ JSON };
use web_sys::{ RtcPeerConnection, RtcConfiguration, RtcSdpType, RtcSessionDescriptionInit };
use wasm_bindgen_futures::JsFuture;
use protocols::WebSocketData;
use crate::{ log, console_log };
use crate::streams::{ Sockets, Data, Socket, State };

#[derive(Serialize, Deserialize)]
struct RTCData {
	r#type: String,
	sdp: String
}

fn get_sdp(obj: &JsValue) -> Option<String> {
	match obj.into_serde::<RTCData>() {
		Ok(obj) => Some(obj.sdp),
		Err(e) => {
			console_log!("error while getting the sdp {:?}", e);
			None
		}
	}
}

/*
[0] : dleft --- left --- right --- dright ^^^^

       { dleft --- [left] --- right --- dright
[1] : {                   \
	   { dleft --- left --- [right] --- dright ^^
		

	   { dleft --- [left] --- {right} --- dright
	                       \
[3] : {  dleft --- [left] -/- [right] --- dright // low geo test ( right and left ) when both are connected 
                           \
	   { dleft --- {left} --- [right] --- dright


[client 1]                    [server]                     [client 2]
  [data]
	|----OfferSDP(data1, None)-->|                               |
	|                            |---OfferSDP(data1, client1)--->| c2 store c1 // server OK
	|                            |<--OfferSDP(data2, client1)---|             // server OK
	|<-OfferSDP(data2, client2)-|                               | c1 store c2 // server OK
	|-IceCandidate(client2)----->|                               |
	|                            |----IceCandidate(client1)----->|
	|                            |<---IceCandidate(client1)------|
	|<---IceCandidate(client2)---|                               |

// TODO real async/await communication protocol
async fn handle_response() {

}

*/


// Doc:
// https://webrtc.org/getting-started/peer-connections
// https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API/Connectivity


const ICE_SERVERS: &str = "[{\"urls\": \"stun:stun.l.google.com:19302\"}]";

// pub struct RTCSocket {

// }

// should return a result
pub async fn create_rtc(socks: &mut Sockets, should_send: bool) {
	if socks.tmp.is_connected() || socks.tmp.is_waiting() {
		console_log!("canot create a new rtc connection, socks.tmp is taken");
		return ;
	}
	let mut conf = RtcConfiguration::new();
	let obj = match JSON::parse(ICE_SERVERS) {
		Ok(res) => res,
		Err(e) => { console_log!("Eroro while parsiong: {:?}", e); return ; }
	};
	conf.ice_servers(&obj);
	console_log!("conf: {:?}", conf);
	let peerConnection = match RtcPeerConnection::new_with_configuration(&conf) {
		Ok(peerConnection) => peerConnection,
		Err(e) => { console_log!("Error while creating peerconnection: {:?}", e); return ;}
	};
	let cb = Closure::wrap(Box::new(move |a: JsValue| {
		console_log!("from icecandidate: {:?}", a);
	}) as Box<dyn FnMut(JsValue)>);
	peerConnection.add_event_listener_with_callback("icecandidate", cb.as_ref().unchecked_ref());
	cb.forget(); // TODO: put the cb in the struct
	socks.tmp.state = State::Waiting(crate::time_now());
	if should_send {
		let offer = match JsFuture::from(peerConnection.create_offer()).await {
			Ok(offer) => offer,
			Err(e) => { console_log!("cannot create the offer: {:?}", e); return ;}
		};
		let message = WebSocketData::OfferSDP(get_sdp(&offer).expect("Cannot create offer string from offer object"), None);
		socks.server.send(Data::WsData(message));
	}
	socks.tmp.socket = Some(Socket::WebRTC(peerConnection));

	// await peerConnection.setLocalDescription(offer);
	// peerConnection.add_event_listener_with_callback("connectionstatechange", cb.as_ref().unchecked_ref()); TODO: this
}

pub async fn incoming_offer(socks: &mut Sockets, sdp: &String, addr: SocketAddr) {
	if socks.tmp.is_connected() || socks.tmp.is_locked(None) { // rly None ?
		console_log!("Icoming SDP but tmp socket already taken and active (should be moved to a non temporary place");
		return ;
	}
	if socks.tmp.socket.is_none() { // should not be true
		create_rtc(socks, false).await; // should handle the result
	}
	let peerConnection = match &socks.tmp.socket {
		Some(Socket::WebRTC(peerConnection)) => peerConnection,
		_ => { console_log!("this should exist"); return ; }
	};
	// this should be done only done one time by socket
	// maybe new state: locked 
	let mut description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
	description.sdp(sdp.as_str()); // offer is an object {type: "...", sdp: "..."} this just need the sdp value
	let res = JsFuture::from(peerConnection.set_remote_description(&description)).await;
	console_log!("remote description return: {:?}", res);
	let answer = match JsFuture::from(peerConnection.create_answer()).await {
		Ok(answer) => get_sdp(&answer).expect("exect arnt writen"),
		Err(e) => {console_log!("error while creating answer: {:?}", e); return ;}
	};
	console_log!("answer: {:?}", answer);
	let mut local_answer = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
	local_answer.sdp(answer.as_str());
	let res = peerConnection.set_local_description(&local_answer);
	console_log!("local description return: {:?}", res);
	let rsp = WebSocketData::AnswerSDP(answer, addr);
	socks.server.send(Data::WsData(rsp));
}

// pub fn respondOffer() {}

// conf: RtcConfiguration {
// 	obj: Object {
// 		obj: JsValue(Object({
// 			"iceServers":"[{'urls': 'stun:stun.l.google.com:19302'}]"})) } }