import React from 'react';

import '~/assets/globals.css';

const Layout: React.FC<{
	children: React.ReactNode;
}> = ({ children }) => {
	return (
		<html lang='en'>
			<body className={`antialiased h-screen w-screen flex`}>{children}</body>
		</html>
	);
};

export default Layout;
