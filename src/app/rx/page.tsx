'use client';

import React from 'react';

export default function Receiver() {
	const audioRef = React.useRef<HTMLAudioElement>(null);
	const pcRef = React.useRef<RTCPeerConnection>(null);
	const wsRef = React.useRef<WebSocket>(null);

	React.useEffect(() => {
		// 1. Create a new RTCPeerConnection.
		const pc = new RTCPeerConnection();
		pcRef.current = pc;

		// 2. Set up ICE candidate handler.
		wsRef.current = new WebSocket('ws://localhost:3001');

		pc.onicecandidate = event => {
			if (event.candidate) {
				wsRef.current.send(JSON.stringify({ candidate: event.candidate }));
			}
		};

		// 3. When remote track arrives, attach it to the audio element.
		pc.ontrack = event => {
			if (audioRef.current) {
				audioRef.current.srcObject = event.streams[0];
			}
		};

		// 4. Listen for signaling messages.
		wsRef.current.onmessage = async (message: MessageEvent) => {
			const data = JSON.parse(await message.data.text());
			if (data.offer) {
				// Set remote description with the received offer.
				await pc.setRemoteDescription(data.offer);
				// Create and send an answer.
				const answer = await pc.createAnswer();
				await pc.setLocalDescription(answer);
				wsRef.current.send(JSON.stringify({ answer }));
			}
			if (data.candidate) {
				await pc.addIceCandidate(data.candidate);
			}
		};
	}, []);

	return (
		<div>
			<h1>Receiver (PC)</h1>
			<audio
				ref={audioRef}
				autoPlay
				controls
			/>
			<p>Waiting for incoming audio...</p>
		</div>
	);
}
