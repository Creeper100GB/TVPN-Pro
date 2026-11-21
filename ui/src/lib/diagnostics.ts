import { countryName } from './countries';
import { STATE_META } from './states';
import type { AppState, SessionSummary, Settings } from './types';

export interface DiagnosticsInput {
	version: string;
	desktop: boolean;
	state: AppState;
	settings: Settings;
	summary: SessionSummary | null;
}

function platform(): string {
	const match = /\(([^)]+)\)/.exec(navigator.userAgent);
	return match?.[1] ?? navigator.platform ?? 'unbekannt';
}

function runtime(): string {
	const ua = navigator.userAgent;
	const edge = /Edg\/([\d.]+)/.exec(ua)?.[1];
	if (edge) return `WebView2 ${edge}`;
	const chrome = /Chrome\/([\d.]+)/.exec(ua)?.[1];
	return chrome ? `Chromium ${chrome}` : 'unbekannt';
}

function yesNo(value: boolean): string {
	return value ? 'an' : 'aus';
}

function settingsLine(settings: Settings): string {
	const torrent = settings.torrentMode === 'vpn_only' ? 'nur VPN' : 'blockiert';
	const mac = settings.macMode === 'original' ? 'Original' : 'neu würfeln';
	const fallback = { off: 'aus', suggest: 'Vorschlag', auto: 'automatisch' }[settings.blockFallback];
	return [
		`VPN ${yesNo(settings.vpnEnabled)}`,
		`Land ${countryName(settings.vpnCountry)}`,
		`Bridges ${yesNo(settings.torBridges)}`,
		`Torrents ${torrent}`,
		`MAC ${mac}`,
		`Fallback ${fallback}`,
		`Hash-Abfrage ${yesNo(settings.scannerHashLookup)}`,
		`Portal ${settings.portalTimeoutSec}s`
	].join(', ');
}

function summaryLine(summary: SessionSummary): string {
	const seconds = Math.round((summary.endedAt - summary.startedAt) / 1000);
	const codes = summary.reasons.length > 0 ? `, Codes ${summary.reasons.join(', ')}` : '';
	return `Letzte Sitzung: ${seconds}s, ${summary.clean} sauber, ${summary.warned} Hinweise, ${summary.quarantined} Quarantäne, ${summary.failed} Fehler, ${summary.failclosedCount} Notfälle${codes}`;
}

/** A short, redaction-safe report that testers can paste into an issue. */
export function formatDiagnostics(input: DiagnosticsInput): string {
	const lines = [
		`TVPN Pro ${input.version} (Alpha)`,
		`Zeit: ${new Date().toISOString()}`,
		`Modus: ${input.desktop ? 'Desktop (Tauri)' : 'Browser-Vorschau'}`,
		`Plattform: ${platform()}`,
		`Laufzeit: ${runtime()}`,
		`Browsersprache: ${navigator.language}`,
		`Fenster: ${window.innerWidth}x${window.innerHeight} @${window.devicePixelRatio}x, ${
			document.documentElement.classList.contains('dark') ? 'dunkel' : 'hell'
		}`,
		`Zustand: ${STATE_META[input.state].label}`,
		`Einstellungen: ${settingsLine(input.settings)}`
	];

	if (input.summary) lines.push(summaryLine(input.summary));
	lines.push('Enthält keine IP-Adressen, MAC-Adressen, URLs oder Dateinamen.');
	return lines.join('\n');
}

/** Clipboard write with a fallback for WebViews without the async API. */
export async function copyText(text: string): Promise<void> {
	try {
		if (navigator.clipboard?.writeText) {
			await navigator.clipboard.writeText(text);
			return;
		}
	} catch {
		// Fall through to the legacy path.
	}

	const area = document.createElement('textarea');
	area.value = text;
	area.setAttribute('readonly', '');
	area.style.position = 'fixed';
	area.style.opacity = '0';
	document.body.appendChild(area);
	area.select();
	const copied = document.execCommand('copy');
	area.remove();
	if (!copied) throw new Error('copy failed');
}
