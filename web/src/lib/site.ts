export interface SiteConfig {
	name: string;
	author: string;
	brand: string;
	description: string;
	footerText: string;
}

export function site(): SiteConfig {
	const name = process.env.PUBLIC_SITE_NAME || 'Blogs';
	const author = process.env.PUBLIC_SITE_AUTHOR ?? 'SnowballSH';
	const brand = author ? `${author} ${name}` : name;
	return {
		name,
		author,
		brand,
		description:
			process.env.PUBLIC_SITE_DESCRIPTION ||
			'Personal and Academic Blogs. Maybe Philosophical. Maybe Mathematical. Or both.',
		footerText: process.env.PUBLIC_FOOTER_TEXT || brand
	};
}
