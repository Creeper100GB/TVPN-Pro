import type { AppState, Tone } from './types';

export interface StateMeta {
	label: string;
	hint: string;
	tone: Tone;
	action: string;
	progress: number;
}

export const STATE_META: Record<AppState, StateMeta> = {
	OFF: {
		label: 'Aus',
		hint: 'Kein Schutz aktiv. Der Rechner nutzt das Netz wie gewohnt.',
		tone: 'neutral',
		action: 'Einschalten',
		progress: 0
	},
	PREFLIGHT: {
		label: 'Vorbereitung',
		hint: 'Prüfe Rechte, Signaturen und Bausteine.',
		tone: 'busy',
		action: 'Läuft …',
		progress: 8
	},
	ARMING: {
		label: 'Firewall scharf',
		hint: 'Blocke alles außer Tor und Freigaben. Die MAC bleibt unverändert.',
		tone: 'busy',
		action: 'Läuft …',
		progress: 24
	},
	PORTAL: {
		label: 'Portal erkannt',
		hint: 'Anmelden nur im VPN-Browser. Das Portal sieht dein Gerät ohne VPN.',
		tone: 'warn',
		action: 'Portal öffnen',
		progress: 34
	},
	VPN_UP: {
		label: 'VPN verbunden',
		hint: 'Tor startet. Die Bindung an den VPN-Adapter ist noch ungeprüft.',
		tone: 'busy',
		action: 'Läuft …',
		progress: 55
	},
	TOR_UP: {
		label: 'Tor startet',
		hint: 'Warte auf Circuit und lokalen Leak-Check.',
		tone: 'busy',
		action: 'Läuft …',
		progress: 78
	},
	READY: {
		label: 'Tor aktiv',
		hint: 'Firewall und Tor laufen. Die VPN-Kette ist ungeprüft; keine Anonymitätsgarantie.',
		tone: 'ok',
		action: 'Ausschalten',
		progress: 100
	},
	DEGRADED: {
		label: 'Eingeschränkt',
		hint: 'Tor läuft ohne verifizierte VPN-Kette. Der ISP sieht Tor-Traffic.',
		tone: 'warn',
		action: 'Ausschalten',
		progress: 88
	},
	FAILCLOSED: {
		label: 'Netz gesperrt',
		hint: 'Etwas ist ausgefallen. Nichts geht raus, bis du entscheidest.',
		tone: 'danger',
		action: 'Notfall-Reset',
		progress: 100
	},
	TEARDOWN: {
		label: 'Wird beendet',
		hint: 'Tor und VPN trennen, Firewall entfernen. Es werden keine Spuren gelöscht.',
		tone: 'busy',
		action: 'Läuft …',
		progress: 40
	},
	RECOVERY: {
		label: 'Wiederherstellung',
		hint: 'Eigene Filter werden entfernt, das Netz wird geprüft.',
		tone: 'busy',
		action: 'Läuft …',
		progress: 60
	}
};

export const TONE_VAR: Record<Tone, string> = {
	neutral: 'var(--muted)',
	busy: 'var(--accent)',
	ok: 'var(--ok)',
	warn: 'var(--warn)',
	danger: 'var(--danger)'
};

export function isTransient(state: AppState): boolean {
	return ['PREFLIGHT', 'ARMING', 'VPN_UP', 'TOR_UP', 'TEARDOWN', 'RECOVERY'].includes(state);
}

export function isArmed(state: AppState): boolean {
	return ['ARMING', 'PORTAL', 'VPN_UP', 'TOR_UP', 'READY', 'DEGRADED', 'FAILCLOSED'].includes(state);
}
