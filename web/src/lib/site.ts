export interface SiteConfig {
	name: string;
	description: string;
	footerText: string;
}

export function site(): SiteConfig {
	const name = process.env.PUBLIC_SITE_NAME || 'snowblog';
	return {
		name,
		description: process.env.PUBLIC_SITE_DESCRIPTION || 'A blog rendered from Typst by snowblog.',
		footerText: process.env.PUBLIC_FOOTER_TEXT || name
	};
}
