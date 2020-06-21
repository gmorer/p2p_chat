use wasm_bindgen::prelude::*;
use futures::channel::mpsc::unbounded;
use futures::future;
use futures::stream::StreamExt;
use js_sys::Date;
use js_sys::Reflect;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    MessageEvent, RtcDataChannelEvent, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
    RtcSessionDescriptionInit,
};

// mod webrtc;
mod streams;
use streams::{ Event, Branch };

mod webrtc;

mod cb;
use cb::CB;

mod html;
use html::connect_html;

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

pub fn time_now() -> u64 {
	Date::new_0().get_time() as u64
}

async fn main_loop() {
	let (sender, mut receiver) = unbounded::<Event>();

	let cb = CB::init(sender.clone());
	connect_html(sender.clone());
	sender.unbounded_send(Event::Disconnect(Branch::Server));
	let mut socks = streams::Sockets::default();
	/*
	for_each not working with async block inside we got:receiver
	"A lifetime cannot be determined in the given situation."
	because of `&mut socks`
	
	receiver.for_each(|e| {
		e.execute(sender.clone(), &mut socks, &cb)
		// future::ready(())
	}).await;
	
	So we use the while loop
	*/
	while let Some(e) = receiver.next().await {
		e.execute(sender.clone(), &mut socks, &cb).await;
	}
	console_log!("This should not be reachable")
}

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
	main_loop().await;
	// test().await;
	// connect_html(&sockets, &document);
	// sockets.main_loop().await;
	// impl on disconnect to reconnect to the server
	Ok(())
}

pub async fn test() -> Result<(), JsValue> {
    /*
     * Set up PeerConnections
     * pc1 <=> pc2
     *
     */
    let pc1 = RtcPeerConnection::new()?;
    console_log!("pc1 created: state {:?}", pc1.signaling_state());
    let pc2 = RtcPeerConnection::new()?;
    console_log!("pc2 created: state {:?}", pc2.signaling_state());

    /*
     * Create DataChannel on pc1 to negotiate
     * Message will be shonw here after connection established
     *
     */
    let dc1 = pc1.create_data_channel("my-data-channel");
    console_log!("dc1 created: label {:?}", dc1.label());

    let dc1_clone = dc1.clone();
    let onmessage_callback =
        Closure::wrap(
            Box::new(move |ev: MessageEvent| match ev.data().as_string() {
                Some(message) => {
                    console_log!("{:?}", message);
                    dc1_clone.send_with_str("Pong from pc1.dc!").unwrap();
                }
                None => {}
            }) as Box<dyn FnMut(MessageEvent)>,
        );
    dc1.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    /*
     * If negotiaion has done, this closure will be called
     *
     */
    let ondatachannel_callback = Closure::wrap(Box::new(move |ev: RtcDataChannelEvent| {
        let dc2 = ev.channel();
        console_log!("pc2.ondatachannel!: {:?}", dc2.label());

        let onmessage_callback =
            Closure::wrap(
                Box::new(move |ev: MessageEvent| match ev.data().as_string() {
                    Some(message) => console_log!("{:?}", message),
                    None => {}
                }) as Box<dyn FnMut(MessageEvent)>,
            );
        dc2.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        dc2.send_with_str("Ping from pc2.dc!").unwrap();
    }) as Box<dyn FnMut(RtcDataChannelEvent)>);
    pc2.set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
    ondatachannel_callback.forget();

    /*
     * Handle ICE candidate each other
     *
     */
    let pc2_clone = pc2.clone();
    let onicecandidate_callback1 =
        Closure::wrap(
            Box::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
                Some(candidate) => {
                    console_log!("pc1.onicecandidate: {:#?}", candidate.candidate());
                    let _ =
                        pc2_clone.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate));
                }
                None => {}
            }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>,
        );
    pc1.set_onicecandidate(Some(onicecandidate_callback1.as_ref().unchecked_ref()));
    onicecandidate_callback1.forget();

    let pc1_clone = pc1.clone();
    let onicecandidate_callback2 =
        Closure::wrap(
            Box::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
                Some(candidate) => {
                    console_log!("pc2.onicecandidate: {:#?}", candidate.candidate());
                    let _ =
                        pc1_clone.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate));
                }
                None => {}
            }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>,
        );
    pc2.set_onicecandidate(Some(onicecandidate_callback2.as_ref().unchecked_ref()));
    onicecandidate_callback2.forget();

    /*
     * Send OFFER from pc1 to pc2
     *
     */
    let offer = JsFuture::from(pc1.create_offer()).await?;
    let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
        .as_string()
        .unwrap();
    console_log!("pc1: offer {:?}", offer_sdp);

    let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_obj.sdp(&offer_sdp);
    let sld_promise = pc1.set_local_description(&offer_obj);
    JsFuture::from(sld_promise).await?;
    console_log!("pc1: state {:?}", pc1.signaling_state());

    /*
     * Receive OFFER from pc1
     * Create and send ANSWER from pc2 to pc1
     *
     */
    let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_obj.sdp(&offer_sdp);
    let srd_promise = pc2.set_remote_description(&offer_obj);
    JsFuture::from(srd_promise).await?;
    console_log!("pc2: state {:?}", pc2.signaling_state());

    let answer = JsFuture::from(pc2.create_answer()).await?;
    let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
        .as_string()
        .unwrap();
    console_log!("pc2: answer {:?}", answer_sdp);

    let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer_obj.sdp(&answer_sdp);
    let sld_promise = pc2.set_local_description(&answer_obj);
    JsFuture::from(sld_promise).await?;
    console_log!("pc2: state {:?}", pc2.signaling_state());

    /*
     * Receive ANSWER from pc2
     *
     */
    let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer_obj.sdp(&answer_sdp);
    let srd_promise = pc1.set_remote_description(&answer_obj);
    JsFuture::from(srd_promise).await?;
    console_log!("pc1: state {:?}", pc1.signaling_state());

    Ok(())
}