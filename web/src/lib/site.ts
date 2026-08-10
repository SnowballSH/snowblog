export interface SiteConfig {
	name: string;
	description: string;
	footerText: string;
}

export function site(): SiteConfig {
	const name = process.env.PUBLIC_SITE_NAME || 'SnowBlog';
	return {
		name,
		description:
			process.env.PUBLIC_SITE_DESCRIPTION ||
			'A blog written in Typst and rendered to fast, clean pages.',
		footerText: process.env.PUBLIC_FOOTER_TEXT || name
	};
}
