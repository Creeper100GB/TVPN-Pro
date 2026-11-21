export type ThemeMode = 'auto' | 'light' | 'dark';

const stored =
	typeof localStorage !== 'undefined' ? (localStorage.getItem('tvpn-theme') as ThemeMode | null) : null;

export const theme = $state<{ mode: ThemeMode }>({
	mode: stored === 'light' || stored === 'dark' || stored === 'auto' ? stored : 'auto'
});

export function setTheme(mode: ThemeMode) {
	theme.mode = mode;
	if (typeof localStorage !== 'undefined') localStorage.setItem('tvpn-theme', mode);
}
