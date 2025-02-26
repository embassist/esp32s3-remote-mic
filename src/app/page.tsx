import React from 'react';
import Link from 'next/link';

const Page: React.FC<{}> = ({}) => {
	return (
		<div className={'flex flex-row items-center justify-center flex-1 gap-2'}>
			<Link
				href={'/tx'}
				className={'border rounded-xl p-2'}
			>
				Share
			</Link>
			<Link
				href={'/rx'}
				className={'border rounded-xl p-2'}
			>
				Listen
			</Link>
		</div>
	);
};

export default Page;
