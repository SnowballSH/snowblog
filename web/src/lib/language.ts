const SHORT_LABELS: Record<string, string> = {
	zh: '中',
	en: 'EN'
};

export function languageLabel(tag: string): string {
	const curated = SHORT_LABELS[tag.toLowerCase()];
	if (curated) return curated;
	try {
		const autonym = new Intl.DisplayNames([tag], { type: 'language' }).of(tag);
		if (autonym && autonym !== tag) return autonym;
	} catch {
		return tag;
	}
	return tag;
}
