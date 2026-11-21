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
	const hit = COUNTRIES.find((c) => c.code === code);
	return hit && hit.code !== 'random' ? hit.name : 'Zufällig';
}
