use std::net::SocketAddr;
use wasm_bindgen::{ JsValue, JsCast };
use wasm_bindgen::prelude::*;
use js_sys::{ JSON, Reflect };
use web_sys::{
	MessageEvent,
	RtcSdpType,
	RtcIceCandidate,
	RtcIceCandidateInit,
	RtcConfiguration,
	RtcPeerConnection,
	RtcPeerConnectionIceEvent,
	RtcDataChannelEvent,
	RtcSessionDescriptionInit,
	RtcDataChannel
};
use wasm_bindgen_futures::JsFuture;
use protocols::{ WebSocketData, IceCandidateStruct };
use crate::{ log, console_log };
use crate::streams::{ Data, Pstream };

// Doc:
// https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API/Connectivity

const ICE_SERVERS: &str = "[{\"urls\": \"stun:stun.l.google.com:19302\"}]";

#[derive(Clone)]
pub struct RTCSocket {
	conn: RtcPeerConnection,
	channel: RtcDataChannel
}

impl RTCSocket {
	pub fn send(&self, data: &String) {
		if let Err(e) = self.channel.send_with_str(data.as_str()) {
			console_log!("error while sending to peer: {:?}", e);
		}
	}

	pub async fn new(server: &Pstream, should_send: bool) -> Result<Self, JsValue> {
		/* Create the RtcPeerConnection struct */
		let mut conf = RtcConfiguration::new();
		let obj = JSON::parse(ICE_SERVERS)?;
		conf.ice_servers(&obj);
		let peer_connection = RtcPeerConnection::new_with_configuration(&conf)?;

		/* Create the Data Channel */
		let data_channel = peer_connection.create_data_channel("my-data-channel");
		let dc_clone = data_channel.clone();
		let onmessage_callback =
			Closure::wrap(
				Box::new(move |ev: MessageEvent| match ev.data().as_string() {
					Some(message) => {
						console_log!("receving: {:?}", message);
						dc_clone.send_with_str("Pong from pc1.dc!").unwrap();
					}
					None => {}
				}) as Box<dyn FnMut(MessageEvent)>,
			);
		data_channel.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
		onmessage_callback.forget();
		let cb = Closure::wrap(Box::new(move || {
			console_log!("Connection close");
		}) as Box<dyn FnMut()>);
		data_channel.set_onclose(Some(cb.as_ref().unchecked_ref()));
		cb.forget();
		let cb = Closure::wrap(Box::new(move || {
			console_log!("Connection open");
		}) as Box<dyn FnMut()>);
		data_channel.set_onopen(Some(cb.as_ref().unchecked_ref()));
		cb.forget();

		/* set the local offer */
		let offer = Reflect::get(&JsFuture::from(peer_connection.create_offer()).await?, &JsValue::from_str("sdp"))?
		.as_string().ok_or(JsValue::from_str("No sdp in the offer"))?;
		let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
		offer_obj.sdp(&offer);
		JsFuture::from(peer_connection.set_local_description(&offer_obj)).await?;
		if should_send {
			let message = WebSocketData::OfferSDP(offer, None);
			server.send(Data::WsData(message));
		}
		Ok(RTCSocket {
			conn: peer_connection,
			channel: data_channel
		})
	}
	
	pub async fn offer(&self, server: &Pstream, sdp: &String, addr: SocketAddr) -> Result<(), JsValue> {
		/* Set Remote offer description */
		let mut description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
		description.sdp(sdp.as_str());
		JsFuture::from(self.conn.set_remote_description(&description)).await?;

		/* Create local answer */
		let answer = Reflect::get(&JsFuture::from(self.conn.create_answer()).await?, &JsValue::from_str("sdp"))?
			.as_string().ok_or("no sdp in answer")?;
		let mut local_answer = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
		local_answer.sdp(answer.as_str());
		JsFuture::from(self.conn.set_local_description(&local_answer)).await?;
		server.send(Data::WsData(WebSocketData::AnswerSDP(answer, addr)));

		/* Handle ice candidate */
		let server = server.clone();
		let cb = Closure::wrap(Box::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
			Some(candidate) => {
				let candidate = IceCandidateStruct {
					candidate: candidate.candidate(),
					sdp_mid: candidate.sdp_mid(),
					sdp_m_line_index: candidate.sdp_m_line_index()
				};
				let message = WebSocketData::IceCandidate(candidate, addr);
				server.send(Data::WsData(message));
			},
			None => {}
		}) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
		self.conn.set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
		cb.forget();

		/* Handle OK connection */
		// TODO: put this in createRTC
		let ondatachannel_callback = Closure::wrap(Box::new(move |ev: RtcDataChannelEvent| {
			let data_channel = ev.channel();
			let onmessage_callback =
				Closure::wrap(
					Box::new(move |ev: MessageEvent| match ev.data().as_string() {
						Some(message) => console_log!("receveing {:?}", message),
						None => {}
					}) as Box<dyn FnMut(MessageEvent)>,
				);
			data_channel.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
			onmessage_callback.forget();

			data_channel.send_with_str("Ping from pc2.dc!").unwrap();
		}) as Box<dyn FnMut(RtcDataChannelEvent)>);
		self.conn.set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
		ondatachannel_callback.forget();
		Ok(())
	}

	pub async fn answer(&self, server: &Pstream, sdp: &String, addr: SocketAddr) -> Result<(), JsValue> {
		let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
		answer_obj.sdp(sdp.as_str());
		JsFuture::from(self.conn.set_remote_description(&answer_obj)).await?;
		/* Handle ice candidate */
		let server = server.clone();
		let cb = Closure::wrap(Box::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
			Some(candidate) => {
				let candidate = IceCandidateStruct {
					candidate: candidate.candidate(),
					sdp_mid: candidate.sdp_mid(),
					sdp_m_line_index: candidate.sdp_m_line_index()
				};
				let message = WebSocketData::IceCandidate(candidate, addr);
				server.send(Data::WsData(message));
			},
			None => {}
		}) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
		self.conn.set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
		cb.forget();
		Ok(())
	}

	pub async fn ice_candidate(&self, candidate: &IceCandidateStruct) -> Result<(), JsValue> {
		let mut icecandidate = RtcIceCandidateInit::new(candidate.candidate.as_str());
		if let Some(sdp_mid) = &candidate.sdp_mid {
			icecandidate.sdp_mid(Some(&sdp_mid.as_str()));
		}
		icecandidate.sdp_m_line_index(candidate.sdp_m_line_index);
	
		let candidate = RtcIceCandidate::new(&icecandidate)?;
		JsFuture::from(self.conn.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate))).await?;
		Ok(())
	}
}