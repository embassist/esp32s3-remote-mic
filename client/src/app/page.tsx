'use client';

import React from 'react';

export default function Home() {
	const [data, setData] = React.useState([]);
	const [port, setPort] = React.useState(null);
	const [reader, setReader] = React.useState(null);

	async function connect() {
		const serialPort = await navigator.serial.requestPort();
		await serialPort.open({ baudRate: 115200 });

		const decoder = new TextDecoderStream();
		const portReader = serialPort.readable.pipeThrough(decoder).getReader();
		setPort(serialPort);
		setReader(portReader);

		while (true) {
			const { value, done } = await portReader.read();
			console.log(value);
			if (done) break;
			setData((prevData) => [...prevData, value]);
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
			<ul className={'list-disc'}>
				{data.map((item, index) => (
					<li key={index} className={'list-item'}>
						{item}
					</li>
				))}
			</ul>
		</div>
	);
}
