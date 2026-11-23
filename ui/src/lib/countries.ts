export const COUNTRIES: Array<{ code: string; name: string }> = [
	{ code: 'random', name: 'Zufällig' },
	{ code: 'at', name: 'Österreich' },
	{ code: 'ca', name: 'Kanada' },
	{ code: 'ch', name: 'Schweiz' },
	{ code: 'cz', name: 'Tschechien' },
	{ code: 'de', name: 'Deutschland' },
	{ code: 'dk', name: 'Dänemark' },
	{ code: 'es', name: 'Spanien' },
	{ code: 'fi', name: 'Finnland' },
	{ code: 'fr', name: 'Frankreich' },
	{ code: 'gb', name: 'Großbritannien' },
	{ code: 'jp', name: 'Japan' },
	{ code: 'nl', name: 'Niederlande' },
	{ code: 'no', name: 'Norwegen' },
	{ code: 'ro', name: 'Rumänien' },
	{ code: 'se', name: 'Schweden' },
	{ code: 'sg', name: 'Singapur' },
	{ code: 'us', name: 'USA' }
];

export function countryName(code: string | null): string {
	if (!code) return 'Nicht verbunden';
	// The service stores ISO codes in upper case, the picker uses lower case.
	const normalized = code.toLowerCase();
	if (normalized === 'random') return 'Zufällig';
	const hit = COUNTRIES.find((c) => c.code === normalized);
	return hit ? hit.name : 'Unbekannt';
}
