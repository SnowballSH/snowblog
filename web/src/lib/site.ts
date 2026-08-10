export interface SiteConfig {
	name: string;
	author: string;
	description: string;
	footerText: string;
}

export function site(): SiteConfig {
	const name = process.env.PUBLIC_SITE_NAME || 'SnowBlog';
	return {
		name,
		author: process.env.PUBLIC_SITE_AUTHOR ?? 'SnowballSH',
		description:
			process.env.PUBLIC_SITE_DESCRIPTION ||
			'Personal and Academic Blogs. Maybe Philosophical. Maybe Mathematical. Or both.',
		footerText: process.env.PUBLIC_FOOTER_TEXT || name
	};
}
