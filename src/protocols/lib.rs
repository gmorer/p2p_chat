// Structure that well be send across the websocket
// in a client-server connection
mod websocket;

pub use websocket::WebSocketData;

// Structure that well be send across the WebRTC
// in a client-client connection
mod webrtc;

pub use webrtc::WebRTCData;
