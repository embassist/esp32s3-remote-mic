'use client';

import React from 'react';

export default function Sender() {
	const audioRef = React.useRef<HTMLAudioElement>(null);
	const pcRef = React.useRef<RTCPeerConnection>(null);
	const wsRef = React.useRef<WebSocket>(null);

	async function tx() {
		const stream = await navigator.mediaDevices.getUserMedia({ audio: true });

		if (audioRef.current) {
			audioRef.current.srcObject = stream;
		}

		// 2. Create a new RTCPeerConnection.
		const pc = new RTCPeerConnection();
		pcRef.current = pc;

		// 3. Add all audio tracks to the connection.
		stream.getTracks().forEach(track => {
			pc.addTrack(track, stream);
		});

		// 4. Set up ICE candidate handler.
		wsRef.current = new WebSocket('ws://localhost:3001');
		wsRef.current.onopen = async () => {
			// Create offer when WebSocket is open.
			const offer = await pc.createOffer();
			await pc.setLocalDescription(offer);
			wsRef.current.send(JSON.stringify({ offer }));
		};

		pc.onicecandidate = event => {
			if (event.candidate) {
				wsRef.current.send(JSON.stringify({ candidate: event.candidate }));
			}
		};

		// 5. Listen for signaling messages.
		wsRef.current.onmessage = async message => {
			const data = JSON.parse(await message.data.text());
			if (data.answer) {
				await pc.setRemoteDescription(data.answer);
			}
			if (data.candidate) {
				await pc.addIceCandidate(data.candidate);
			}
		};
	}

	React.useEffect(() => {
		if (!navigator) return;
		tx().then(() => console.log('shared'));
	}, []);

	return (
		<div>
			<h1>Sender (Phone)</h1>
			{/* Audio element to monitor local stream (muted to avoid feedback) */}
			<audio
				ref={audioRef}
				autoPlay
				muted
				controls
			/>
			<p>Your audio is being sent to the receiver...</p>
		</div>
	);
}
