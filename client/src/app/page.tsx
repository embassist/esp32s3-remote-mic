'use client';

import React from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip } from "recharts";

export default function Home() {
	const [port, setPort] = React.useState(null);
	const [reader, setReader] = React.useState(null);
	const audioRef = React.useRef<HTMLAudioElement>(null);

	async function connect() {
		const serialPort = await navigator.serial.requestPort();
		await serialPort.open({ baudRate: 115200 });

		const decoder = new TextDecoderStream();
		const portReader = serialPort.readable.pipeThrough(decoder).getReader();
		setPort(serialPort);
		setReader(portReader);

		const mediaSource = new MediaSource();
		audioRef.current.src = URL.createObjectURL(mediaSource);

		let sourceBuffer: SourceBuffer;

		mediaSource.addEventListener("sourceopen", () => {
			sourceBuffer = mediaSource.addSourceBuffer('audio/wav;');
		});

		while (true) {
			const { value, done } = await portReader.read();
			if (done) break;

			if (sourceBuffer && !sourceBuffer.updating) {
				let chunk = new Uint8Array(value);
				console.log(chunk.length);
				sourceBuffer.appendBuffer(chunk);
			}
		}
	}

	async function disconnect() {
			if (reader) {
				await reader.cancel();
				reader.releaseLock();
			}
			if (port) {
				await port.close();
				setPort(null);
				setReader(null);
			}
	}

	return (
		<div className={'flex flex-col gap-2'}>
			<div className={'flex flex-row gap-2'}>
				<button onClick={connect}>
					Connect
				</button>
				<button onClick={disconnect}>
					Disconnect
				</button>
			</div>
			<audio ref={audioRef} controls></audio>
		</div>
	);
}
