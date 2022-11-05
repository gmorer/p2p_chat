use crate::event::Event;
use crate::html::Html;
use crate::streams::{Data, Pstream};
use crate::{console_log, slog, Sender};
use crossplatform::id::Id;
use crossplatform::proto_ws::{AnswerSDP, IceCandidateStruct, OfferSDP, WebSocketData};
use futures::channel::mpsc::UnboundedSender;
use futures::channel::oneshot;
use js_sys::{Reflect, JSON};
use std::net::SocketAddr;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelEvent, RtcDataChannelType,
    RtcIceCandidate, RtcIceCandidateInit, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
    RtcSessionDescriptionInit,
};

const ICE_SERVERS: &str = "[{\"urls\": \"stun:stun.l.google.com:19302\"}]";

enum Error {
    JsError(JsValue),
}

impl From<JsValue> for Error {
    fn from(err: JsValue) -> Self {
        Self::JsError(err)
    }
}

pub struct RTCStreamBuilder {
    // Some cbs
// offer: Option<OfferSDP>,
}

enum SDP {
    Offer(OfferSDP),
    Answer(AnswerSDP),
}

pub struct RTCStream {}

impl RTCStreamBuilder {
    pub async fn new(
        remote_offer: Option<OfferSDP>,
        receiver: oneshot::Receiver<SDP>,
        sender: oneshot::Sender<SDP>,
        on_ice_candidate: UnboundedSender<IceCandidateStruct>,
    ) -> Result<Self, Error> {
        let mut conf = RtcConfiguration::new();
        let obj = JSON::parse(ICE_SERVERS)?;
        conf.ice_servers(&obj);
        let peer_connection = RtcPeerConnection::new_with_configuration(&conf)?;
        /* Create the Data Channel */
        if remote_offer.is_none() {
            let data_channel = peer_connection.create_data_channel("my-data-channel");
            data_channel.set_binary_type(RtcDataChannelType::Arraybuffer);
        }
        if let Some(remote_offer) = remote_offer {
            /* set the remote offer */
            let mut description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
            description.sdp(remote_offer.0.as_str());
            JsFuture::from(peer_connection.set_remote_description(&description)).await?;
            /* Create the answer */
            /* Create local answer */
            let answer = Reflect::get(
                &JsFuture::from(peer_connection.create_answer()).await?,
                &JsValue::from_str("sdp"),
            )?
            .as_string()
            .ok_or(JsValue::from_str("no sdp in answer"))?;
            let mut local_answer = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
            local_answer.sdp(answer.as_str());
            JsFuture::from(peer_connection.set_local_description(&local_answer)).await?;
            sender.send(SDP::Answer(AnswerSDP(answer)));
        } else {
            /* set the local offer */
            let local_offer = Reflect::get(
                &JsFuture::from(peer_connection.create_offer()).await?,
                &JsValue::from_str("sdp"),
            )?
            .as_string()
            .ok_or(JsValue::from_str("No sdp in the offer"))?;
            let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
            offer_obj.sdp(&local_offer);
            JsFuture::from(peer_connection.set_local_description(&offer_obj)).await?;
            sender.send(SDP::Offer(OfferSDP(local_offer)));
            // Some(OfferSDP(local_offer))
        }

        /* Create the Data Channel */
        Ok(Self {})
    }

    pub fn get_offer(&self) -> Option<OfferSDP> {
        self.offer
    }

    pub fn set_ice_candidate(candidate: IceCandidateStruct) {}
    pub async fn get_ice_candidate() -> Option<IceCandidateStruct> {
        None
    }

    pub fn connect() -> RTCStream {
        RTCStream {}
    }
}

impl RTCStream {
    pub fn send_sync() {}
}

#[derive(Debug)]
pub struct RTCSocket {
    conn: RtcPeerConnection,
    pub channel: RtcDataChannel,
    pub cbs: Vec<Closure<dyn FnMut(JsValue)>>, // keep the callbacks in memory
}

impl Clone for RTCSocket {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
            channel: self.channel.clone(),
            cbs: vec![], // callbacks need to be in online one place
        }
    }
}

impl RTCSocket {
    pub fn send(&self, data: &[u8]) {
        console_log!("sending: {:?}", data);
        if let Err(e) = self.channel.send_with_u8_array(data) {
            console_log!("error while sending to peer: {:?}", e);
        }
    }

    pub async fn new(
        server: &Pstream,
        sender: Sender,
        html: &Html,
        should_send: bool,
    ) -> Result<Self, JsValue> {
        let mut cbs = vec![];
        /* Create the RtcPeerConnection struct */
        let mut conf = RtcConfiguration::new();
        let obj = JSON::parse(ICE_SERVERS)?;
        conf.ice_servers(&obj);
        let peer_connection = RtcPeerConnection::new_with_configuration(&conf)?;

        /* Create the Data Channel */
        let data_channel = peer_connection.create_data_channel("my-data-channel");
        data_channel.set_binary_type(RtcDataChannelType::Arraybuffer);
        // let dc_clone = data_channel.clone();
        let onmessage_callback =
            Closure::wrap(Box::new(move |ev: JsValue| {
                match MessageEvent::from(ev).data().as_string() {
                    Some(message) => sender.send(Event::TmpId(message)),
                    None => {}
                }
            }) as Box<dyn FnMut(JsValue)>);
        data_channel.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        cbs.push(onmessage_callback);

        /* set the local offer */
        let offer = Reflect::get(
            &JsFuture::from(peer_connection.create_offer()).await?,
            &JsValue::from_str("sdp"),
        )?
        .as_string()
        .ok_or(JsValue::from_str("No sdp in the offer"))?;
        let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_obj.sdp(&offer);
        JsFuture::from(peer_connection.set_local_description(&offer_obj)).await?;
        if should_send {
            let message = WebSocketData::OfferSDP(offer, None);
            html.chat_info("Asking the server for a peer...");
            server.send(Data::WsData(message));
        }
        Ok(RTCSocket {
            conn: peer_connection,
            channel: data_channel,
            cbs,
        })
    }

    pub async fn offer(
        &mut self,
        server: &Pstream,
        sdp: &String,
        addr: SocketAddr,
        sender: Sender,
    ) -> Result<(), JsValue> {
        /* Set Remote offer description */
        let mut description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        description.sdp(sdp.as_str());
        JsFuture::from(self.conn.set_remote_description(&description)).await?;

        /* Create local answer */
        let answer = Reflect::get(
            &JsFuture::from(self.conn.create_answer()).await?,
            &JsValue::from_str("sdp"),
        )?
        .as_string()
        .ok_or("no sdp in answer")?;
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
                        sdp_m_line_index: candidate.sdp_m_line_index(),
                    };
                    let message = WebSocketData::IceCandidate(candidate, addr);
                    server.send(Data::WsData(message));
                }
                None => {}
            }
        }) as Box<dyn FnMut(JsValue)>);
        self.conn
            .set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
        self.cbs.push(cb);

        /* Handle OK connection */
        let ondatachannel_callback = Closure::wrap(Box::new(move |ev: JsValue| {
            let channel = RtcDataChannelEvent::from(ev).channel();
            channel.set_binary_type(RtcDataChannelType::Arraybuffer);
            sender.send(Event::DCObj(channel));
        }) as Box<dyn FnMut(JsValue)>);
        self.conn
            .set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
        self.cbs.push(ondatachannel_callback);
        Ok(())
    }

    pub async fn answer(
        &mut self,
        server: &Pstream,
        sdp: &String,
        addr: SocketAddr,
        sender: Sender,
        id: Id,
    ) -> Result<(), JsValue> {
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
                        sdp_m_line_index: candidate.sdp_m_line_index(),
                    };
                    let message = WebSocketData::IceCandidate(candidate, addr);
                    server.send(Data::WsData(message));
                }
                None => {}
            }
        }) as Box<dyn FnMut(JsValue)>);
        self.conn
            .set_onicecandidate(Some(cb.as_ref().unchecked_ref()));
        self.cbs.push(cb);
        let sender_cl = sender.clone();
        let cb = Closure::wrap(Box::new(move |_arg: JsValue| {
            sender_cl.send(Event::RtcState(false));
        }) as Box<dyn FnMut(JsValue)>);
        console_log!("Put false onclose");
        self.channel.set_onclose(Some(cb.as_ref().unchecked_ref()));
        self.cbs.push(cb);
        let dc_clone = self.channel.clone();
        let cb = Closure::wrap(Box::new(move |_arg: JsValue| {
            sender.send(Event::RtcState(true));
            if let Err(e) = dc_clone.send_with_str(id.to_name().as_str()) {
                console_log!("error while sending to peer: {:?}", e);
            }
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
        JsFuture::from(
            self.conn
                .add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate)),
        )
        .await?;
        Ok(())
    }

    // Data channel
    pub fn set_dc(&mut self, dc: RtcDataChannel, sender: Sender, id: Id) -> Result<(), JsValue> {
        let sender_cl = sender.clone();
        let onmessage_callback =
            Closure::wrap(Box::new(move |ev: JsValue| {
                match MessageEvent::from(ev).data().as_string() {
                    Some(message) => sender_cl.send(Event::TmpId(message)),
                    None => {}
                }
            }) as Box<dyn FnMut(JsValue)>);
        console_log!("Put false onmessage");
        dc.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        self.cbs.push(onmessage_callback);

        let sender_cl = sender.clone();
        let cb = Closure::wrap(Box::new(move |_arg: JsValue| {
            sender_cl.send(Event::RtcState(false));
        }) as Box<dyn FnMut(JsValue)>);
        console_log!("Put false onclose");
        dc.set_onclose(Some(cb.as_ref().unchecked_ref()));
        self.cbs.push(cb);
        let dc_clone = dc.clone();
        let cb = Closure::wrap(Box::new(move |_arg: JsValue| {
            sender.send(Event::RtcState(true));
            if let Err(e) = dc_clone.send_with_str(id.to_name().as_str()) {
                // set id
                console_log!("error while sending to peer: {:?}", e);
            }
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
