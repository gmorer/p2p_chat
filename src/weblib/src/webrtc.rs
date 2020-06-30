use std::net::SocketAddr;
use wasm_bindgen::{ JsValue, JsCast };
use wasm_bindgen::closure::Closure;
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
use crossplatform::proto::{ WebSocketData, IceCandidateStruct };
use crate::{ log, console_log, Sender };
use crate::streams::{ Data, Pstream, Event };

const ICE_SERVERS: &str = "[{\"urls\": \"stun:stun.l.google.com:19302\"}]";

pub struct RTCSocket {
	conn: RtcPeerConnection,
	channel: RtcDataChannel,
	cbs: Vec<Closure<dyn FnMut (JsValue)>> // keep the callbacks in memory
}

impl Clone for RTCSocket {
	fn clone(&self) -> Self {
		Self {
			conn: self.conn.clone(),
			channel: self.channel.clone(),
			cbs: vec!() // callbacks need to be in online one place
		}
	}
}

impl RTCSocket {
	pub fn send(&self, data: &String) {
		if let Err(e) = self.channel.send_with_str(data.as_str()) {
			console_log!("error while sending to peer: {:?}", e);
		}
	}

	pub async fn new(server: &Pstream, should_send: bool) -> Result<Self, JsValue> {
		let mut cbs = vec!();
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
			Box::new(move |ev: JsValue| {
				match MessageEvent::from(ev).data().as_string() {
					Some(message) => {
						console_log!("receving: {:?}", message);
						dc_clone.send_with_str("Pong from pc1.dc!").unwrap();
					}
					None => {}
				}
			}) as Box<dyn FnMut(JsValue)>,
		);
		data_channel.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
		cbs.push(onmessage_callback);

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
			channel: data_channel,
			cbs
		})
	}
	
	pub async fn offer(&mut self, server: &Pstream, sdp: &String, addr: SocketAddr, sender: Sender) -> Result<(), JsValue> {
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
		let cb = Closure::wrap(Box::new(move |ev: JsValue| {
			match RtcPeerConnectionIceEvent::from(ev).candidate() {
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
			}
		}) as Box<dyn FnMut(JsValue)>);
		self.conn.set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
		self.cbs.push(cb);

		/* Handle OK connection */
		let ondatachannel_callback = Closure::wrap(Box::new(move |ev: JsValue| {
			sender.send(Event::DCObj(RtcDataChannelEvent::from(ev).channel()))
		}) as Box<dyn FnMut(JsValue)>);
		self.conn.set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
		self.cbs.push(ondatachannel_callback);
		Ok(())
	}

	pub async fn answer(&mut self, server: &Pstream, sdp: &String, addr: SocketAddr, sender: Sender) -> Result<(), JsValue> {
		let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
		answer_obj.sdp(sdp.as_str());
		JsFuture::from(self.conn.set_remote_description(&answer_obj)).await?;
		/* Handle ice candidate */
		let server = server.clone();
		let cb = Closure::wrap(Box::new(move |ev: JsValue| {
			match RtcPeerConnectionIceEvent::from(ev).candidate() {
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
			}
		}) as Box<dyn FnMut(JsValue)>);
		self.conn.set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
		self.cbs.push(cb);
		let sender_cl = sender.clone();
		let cb = Closure::wrap(Box::new(move |_arg: JsValue| {
			sender_cl.send(Event::RTCState(false));
		}) as Box<dyn FnMut(JsValue)>);
		self.channel.set_onclose(Some(cb.as_ref().unchecked_ref()));
		self.cbs.push(cb);
		let cb = Closure::wrap(Box::new(move |_arg: JsValue| {
			sender.send(Event::RTCState(true))
		}) as Box<dyn FnMut(JsValue)>);
		self.channel.set_onopen(Some(cb.as_ref().unchecked_ref()));
		self.cbs.push(cb);
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

	pub fn set_dc(&mut self, dc: RtcDataChannel, sender: Sender) -> Result<(), JsValue> {
		let onmessage_callback =
			Closure::wrap(
				Box::new(move |ev: JsValue| {
					match MessageEvent::from(ev).data().as_string() {
						Some(message) => console_log!("Receiving: {:?}", message),
						None => {}
					}
				}) as Box<dyn FnMut(JsValue)>,
			);
		dc.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
		self.cbs.push(onmessage_callback);

		let sender_cl = sender.clone();
		let cb = Closure::wrap(Box::new(move |_arg: JsValue| {
			sender_cl.send(Event::RTCState(false));
		}) as Box<dyn FnMut(JsValue)>);
		dc.set_onclose(Some(cb.as_ref().unchecked_ref()));
		self.cbs.push(cb);
		
		dc.send_with_str("Ping from pc2.dc!").unwrap();
		let cb = Closure::wrap(Box::new(move |_arg: JsValue| {
			sender.send(Event::RTCState(true));
		}) as Box<dyn FnMut(JsValue)>);
		dc.set_onopen(Some(cb.as_ref().unchecked_ref()));
		self.cbs.push(cb);

		self.channel = dc;
		Ok(())
	}

	pub fn delete(&self) {
		self.channel.close();
		self.conn.close();
	}
}