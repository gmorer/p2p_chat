use std::net::SocketAddr;
use wasm_bindgen::{ JsValue, JsCast };
use wasm_bindgen::prelude::*;
use js_sys::{ JSON, Reflect };
use web_sys::{ RtcPeerConnection, RtcConfiguration, RtcSdpType, RtcSessionDescriptionInit, RtcPeerConnectionIceEvent, MessageEvent, RtcDataChannelEvent, RtcIceCandidate, RtcIceCandidateInit };
use wasm_bindgen_futures::JsFuture;
use protocols::{ WebSocketData, IceCandidateStruct };
use crate::{ log, console_log };
use crate::streams::{ Sockets, Data, Socket, State };

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

	/* Message handler : pong */
	let data_channel = peerConnection.create_data_channel("my-data-channel");
	console_log!("dc1 created: label {:?}", data_channel.label());
	let dc_clone = data_channel.clone();
	let onmessage_callback =
		Closure::wrap(
			Box::new(move |ev: MessageEvent| match ev.data().as_string() {
				Some(message) => {
					console_log!("{:?}", message);
					dc_clone.send_with_str("Pong from pc1.dc!").unwrap();
				}
				None => {}
			}) as Box<dyn FnMut(MessageEvent)>,
		);
	data_channel.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
	onmessage_callback.forget();

	
	// let cb = Closure::wrap(Box::new(|event: JsValue| {
	// 	console_log!("new state {:?}", event);
	// }) as Box<dyn FnMut(JsValue)>);
	// peerConnection.add_event_listener_with_callback("connectionstatechange", cb.as_ref().unchecked_ref());
	// cb.forget(); // TODO: put the cb in the struct
	if should_send {
		let offer = match JsFuture::from(peerConnection.create_offer()).await {
			Ok(offer) => Reflect::get(&offer, &JsValue::from_str("sdp")).expect("no sdp in offer").as_string().unwrap(),
			Err(e) => { console_log!("cannot create the offer: {:?}", e); return ;}
		};
		let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
		offer_obj.sdp(&offer);
		JsFuture::from(peerConnection.set_local_description(&offer_obj)).await;
		let message = WebSocketData::OfferSDP(offer, None);
		socks.server.send(Data::WsData(message));
	}
	console_log!("peer connection state: {:?}", peerConnection.signaling_state());
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
	console_log!("peer connection state: {:?}", peerConnection.signaling_state());
	let answer = match JsFuture::from(peerConnection.create_answer()).await {
		Ok(answer) => Reflect::get(&answer, &JsValue::from_str("sdp")).expect("no sdp in answer").as_string().unwrap(),
		Err(e) => {console_log!("error while creating answer: {:?}", e); return ;}
	};
	console_log!("answer: {:?}", answer);
	let mut local_answer = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
	local_answer.sdp(answer.as_str());
	socks.tmp.state = State::Locked(addr);
	let res = JsFuture::from(peerConnection.set_local_description(&local_answer)).await;
	console_log!("peer connection state {:?}", peerConnection.signaling_state());
	let rsp = WebSocketData::AnswerSDP(answer, addr);
	socks.server.send(Data::WsData(rsp));
	/* Handle ice candidate */
	let server = socks.server.clone();
	let cb = Closure::wrap(Box::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
		Some(candidate) => {
			console_log!("sending candidate: {}", candidate.candidate());
			let candidate = IceCandidateStruct {
				candidate: candidate.candidate(),
				sdp_mid: candidate.sdp_mid(),
				sdp_m_line_index: candidate.sdp_m_line_index()
			};
			let message = WebSocketData::IceCandidate(candidate, addr);
			server.send(Data::WsData(message));
			// console_log!("pc.onicecandidate: {:?}", candidate.candidate());
			// pc_clone.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate));
		},
		None => {}
	}) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
	peerConnection.set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
	cb.forget();
	/* Handle OK connection */
	let ondatachannel_callback = Closure::wrap(Box::new(move |ev: RtcDataChannelEvent| {
		let data_channel = ev.channel();
		console_log!("pc2.ondatachannel!: {:?}", data_channel.label());

		let onmessage_callback =
			Closure::wrap(
				Box::new(move |ev: MessageEvent| match ev.data().as_string() {
					Some(message) => console_log!("{:?}", message),
					None => {}
				}) as Box<dyn FnMut(MessageEvent)>,
			);
		data_channel.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
		onmessage_callback.forget();

		data_channel.send_with_str("Ping from pc2.dc!").unwrap();
	}) as Box<dyn FnMut(RtcDataChannelEvent)>);
	peerConnection.set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
	ondatachannel_callback.forget();
}

pub async fn incomming_answer(socks: &mut Sockets, sdp: &String, addr: SocketAddr) {
	if socks.tmp.is_locked(None) {
		console_log!("The socket is locked");
		return ;
	}
	let peerConnection = match &socks.tmp.socket {
		Some(Socket::WebRTC(peerConnection)) => peerConnection,
		_ => { console_log!("this should exist"); return ; }
	};
	socks.tmp.state = State::Locked(addr);
	let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
	answer_obj.sdp(sdp.as_str());
	JsFuture::from(peerConnection.set_remote_description(&answer_obj)).await;
	console_log!("pc1: state {:?}", peerConnection.signaling_state());
	/* Handle ice candidate */
	let server = socks.server.clone();
	let cb = Closure::wrap(Box::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
		Some(candidate) => {
			console_log!("sending candidate: {}", candidate.candidate());
			let candidate = IceCandidateStruct {
				candidate: candidate.candidate(),
				sdp_mid: candidate.sdp_mid(),
				sdp_m_line_index: candidate.sdp_m_line_index()
			};
			let message = WebSocketData::IceCandidate(candidate, addr);
			server.send(Data::WsData(message));
			// console_log!("pc.onicecandidate: {:?}", candidate.candidate());
			// pc_clone.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate));
			// console_log!("Done");
		},
		None => {}
	}) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
	peerConnection.set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
	cb.forget();
}

pub async fn incomming_ice_candidate(socks: &mut Sockets, candidate: &IceCandidateStruct, addr: SocketAddr) {
	if !socks.tmp.is_locked(Some(addr)) {
		console_log!("The socket should be locked");
		return ;
	}
	let peerConnection = match &socks.tmp.socket {
		Some(Socket::WebRTC(peerConnection)) => peerConnection,
		_ => { console_log!("this should exist"); return ; }
	};
	let mut icecandidate = RtcIceCandidateInit::new(candidate.candidate.as_str());
	if let Some(sdp_mid) = &candidate.sdp_mid {
		icecandidate.sdp_mid(Some(&sdp_mid.as_str()));
	}
	icecandidate.sdp_m_line_index(candidate.sdp_m_line_index);
	// if let Some(sdp_m_line_index) = candidate.sdp_m_line_index {
	// }
	match RtcIceCandidate::new(&icecandidate) {
		Ok(candidate) => peerConnection.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate)),
		Err(e) => { console_log!("Error while creating ice candidate: {:?}", e); return }
	};
}
// pub fn respondOffer() {}

// conf: RtcConfiguration {
// 	obj: Object {
// 		obj: JsValue(Object({
// 			"iceServers":"[{'urls': 'stun:stun.l.google.com:19302'}]"})) } }