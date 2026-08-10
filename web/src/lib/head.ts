export interface Alternate {
	lang: string;
	href: string;
}

export interface PostHead {
	canonical: string;
	alternates: Alternate[];
}

export function postAlternates(
	origin: string,
	slug: string,
	languages: string[],
	defaultLanguage: string
): PostHead {
	const base = `${origin}/posts/${encodeURIComponent(slug)}`;
	return {
		canonical: base,
		alternates: languages.map((lang) => ({
			lang,
			href: lang === defaultLanguage ? base : `${base}?lang=${encodeURIComponent(lang)}`
		}))
	};
}
