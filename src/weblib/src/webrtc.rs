
/*
	From MDN, step by step:
	1 - The caller captures local Media via navigator.mediaDevices.getUserMedia() 
	2 - The caller creates RTCPeerConnection and called RTCPeerConnection.addTrack() (Since addStream is deprecating)
	3 - The caller calls RTCPeerConnection.createOffer() to create an offer.
	4 - The caller calls RTCPeerConnection.setLocalDescription() to set that offer as the local description (that is, the description of the local end of the connection).
	5 - After setLocalDescription(), the caller asks STUN servers to generate the ice candidates
	6 - The caller uses the signaling server to transmit the offer to the intended receiver of the call.
	7 - The recipient receives the offer and calls RTCPeerConnection.setRemoteDescription() to record it as the remote description (the description of the other end of the connection).
	8 - The recipient does any setup it needs to do for its end of the call: capture its local media, and attach each media tracks into the peer connection via RTCPeerConnection.addTrack()
	9 - The recipient then creates an answer by calling RTCPeerConnection.createAnswer().
	10 - The recipient calls RTCPeerConnection.setLocalDescription(), passing in the created answer, to set the answer as its local description. The recipient now knows the configuration of both ends of the connection.
	11 - The recipient uses the signaling server to send the answer to the caller.
	12 - The caller receives the answer.
	13 - The caller calls RTCPeerConnection.setRemoteDescription() to set the answer as the remote description for its end of the call. It now knows the configuration of both peers. Media begins to flow as configured.
*/


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