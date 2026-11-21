import { COUNTRIES } from './countries';
import { STATE_META } from './states';
import type {
	AppState,
	BannerKind,
	DownloadItem,
	DownloadRoute,
	DownloadSource,
	ScanVerdict,
	Settings,
	SessionSummary,
	WatchdogLevel
} from './types';

interface AppStore {
	state: AppState;
	since: number;
	progress: number;
	country: string | null;
	vpnConnected: boolean;
	torBootstrap: number;
	circuit: boolean;
	macRandomized: boolean;
	firewallArmed: boolean;
	watchdog: WatchdogLevel;
	banners: BannerKind[];
	downloads: DownloadItem[];
	settings: Settings;
	demoPortal: boolean;
	failureCode: string | null;
	lastSummary: SessionSummary | null;
	summaryOpen: boolean;
	settingsOpen: boolean;
	downloadsOpen: boolean;
	toast: string | null;
}

const defaultSettings: Settings = {
	vpnEnabled: true,
	vpnCountry: 'random',
	torBridges: false,
	blockFallback: 'auto',
	macMode: 'random',
	torrentMode: 'blocked',
	clipboardWatch: false,
	scannerHashLookup: false,
	portalTimeoutSec: 180
};

export const app = $state<AppStore>({
	state: 'OFF',
	since: Date.now(),
	progress: 0,
	country: null,
	vpnConnected: false,
	torBootstrap: 0,
	circuit: false,
	macRandomized: false,
	firewallArmed: false,
	watchdog: 'ok',
	banners: [],
	downloads: [],
	settings: { ...defaultSettings },
	demoPortal: false,
	failureCode: null,
	lastSummary: null,
	summaryOpen: false,
	settingsOpen: false,
	downloadsOpen: false,
	toast: null
});

let generation = 0;
let toastTimer: ReturnType<typeof setTimeout> | null = null;

function sleep(ms: number) {
	return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

function setState(state: AppState, progress?: number) {
	app.state = state;
	app.since = Date.now();
	app.progress = progress ?? STATE_META[state].progress;
}

function pickCountry(): string {
	const pool = COUNTRIES.filter((c) => c.code !== 'random').map((c) => c.code);
	return pool[Math.floor(Math.random() * pool.length)];
}

function addBanner(kind: BannerKind) {
	if (!app.banners.includes(kind)) app.banners = [...app.banners, kind];
}

function removeBanner(kind: BannerKind) {
	app.banners = app.banners.filter((b) => b !== kind);
}

export function showToast(message: string) {
	app.toast = message;
	if (toastTimer) clearTimeout(toastTimer);
	toastTimer = setTimeout(() => (app.toast = null), 4000);
}

/** How many downloads are queued, running or paused. */
export function activeDownloadCount(): number {
	return app.downloads.filter(
		(item) => item.phase === 'queued' || item.phase === 'active' || item.phase === 'paused'
	).length;
}

/** How many downloads ended up in quarantine. */
export function quarantinedDownloadCount(): number {
	return app.downloads.filter((item) => item.verdict === 'quarantined').length;
}

function summarizeDownloads() {
	let clean = 0;
	let warned = 0;
	let quarantined = 0;
	let failed = 0;

	for (const item of app.downloads) {
		switch (item.verdict) {
			case 'clean':
				clean += 1;
				break;
			case 'warned':
				warned += 1;
				break;
			case 'quarantined':
				quarantined += 1;
				break;
			case 'failed':
				failed += 1;
				break;
			case null:
				break;
		}
	}

	return { clean, warned, quarantined, failed };
}

/** Contract rules from `contracts/ipc-v0.md` section 7, checked in the UI too. */
export function validateDownloadUrl(raw: string): string | null {
	let parsed: URL;
	try {
		parsed = new URL(raw);
	} catch {
		return 'Das ist keine gültige URL.';
	}

	if (parsed.protocol !== 'https:') return 'Nur HTTPS ist erlaubt.';
	if (parsed.port && parsed.port !== '443') return 'Nur Port 443 ist erlaubt.';
	if (parsed.username || parsed.password) return 'Keine Zugangsdaten in der URL.';
	if (parsed.hostname.startsWith('[') || /^\d{1,3}(\.\d{1,3}){3}$/.test(parsed.hostname)) {
		return 'IP-Adressen sind als Ziel nicht erlaubt.';
	}
	if (parsed.hostname === 'localhost' || parsed.hostname.endsWith('.local')) {
		return 'Lokale Namen sind nicht erlaubt.';
	}
	return null;
}

export interface ParsedDownload {
	ok: boolean;
	error: string | null;
	label: string;
	source: DownloadSource;
}

/** Validates the input exactly like the service will. */
export function parseDownloadInput(raw: string, route: DownloadRoute): ParsedDownload {
	const value = raw.trim();

	if (route === 'vpn' && app.settings.torrentMode !== 'vpn_only') {
		return {
			ok: false,
			error: 'Die VPN-Route ist ausgeschaltet. Aktiviere sie in den Einstellungen.',
			label: '',
			source: 'https'
		};
	}

	const isMagnet = value.toLowerCase().startsWith('magnet:');
	if (isMagnet) {
		if (route !== 'vpn') {
			return { ok: false, error: 'Torrents laufen nur über die VPN-Route.', label: '', source: 'torrent' };
		}
		const magnetError = validateMagnet(value);
		if (magnetError) {
			return { ok: false, error: magnetError, label: '', source: 'torrent' };
		}
		return { ok: true, error: null, label: magnetLabel(value), source: 'torrent' };
	}

	const urlError = validateDownloadUrl(value);
	if (urlError) {
		return { ok: false, error: urlError, label: '', source: 'https' };
	}
	return { ok: true, error: null, label: labelFromUrl(value), source: 'https' };
}

export function addDownload(
	raw: string,
	route: DownloadRoute = 'tor'
): { ok: boolean; error: string | null } {
	const parsed = parseDownloadInput(raw, route);
	if (!parsed.ok) return { ok: false, error: parsed.error };

	const id = crypto.randomUUID().slice(0, 8);
	app.downloads.push({
		id,
		label: parsed.label,
		phase: 'queued',
		progress: 0,
		verdict: null,
		route,
		source: parsed.source
	});
	if (route === 'vpn') addBanner('vpn_route');
	void runDownload(id);
	return { ok: true, error: null };
}

/** Adds an item that the Tauri backend will drive. */
export function trackDownload(item: DownloadItem) {
	app.downloads.push(item);
}

export function patchDownload(id: string, patch: Partial<DownloadItem>) {
	const item = app.downloads.find((entry) => entry.id === id);
	if (item) Object.assign(item, patch);
}

function labelFromUrl(raw: string): string {
	const parsed = new URL(raw);
	return decodeURIComponent(parsed.pathname.split('/').filter(Boolean).pop() ?? 'download.bin');
}

function magnetLabel(raw: string): string {
	const match = /[?&]dn=([^&]+)/i.exec(raw);
	if (!match) return 'Torrent-Image';
	try {
		return decodeURIComponent(match[1].replace(/\+/g, ' '));
	} catch {
		return 'Torrent-Image';
	}
}

/** Mirrors the contract's magnet rule: only `xt=urn:btih:` with 40 hex or 32 base32. */
export function validateMagnet(raw: string): string | null {
	if (!raw.toLowerCase().startsWith('magnet:?')) return 'Das ist kein Magnet-Link.';
	const hash = /(?:^|&)xt=urn:btih:([^&]+)/i.exec(raw)?.[1] ?? '';
	const hex = /^[0-9a-f]{40}$/i.test(hash);
	const base32 = /^[a-z2-7]{32}$/i.test(hash);
	return hex || base32 ? null : 'Der Info-Hash fehlt oder ist ungültig.';
}

async function runDownload(id: string) {
	await sleep(400);
	if (!app.downloads.some((item) => item.id === id)) return;

	for (const progress of [18, 42, 67, 88, 100]) {
		const item = app.downloads.find((entry) => entry.id === id);
		if (!item) return;
		item.phase = 'active';
		item.progress = progress;
		await sleep(300);
	}

	const finished = app.downloads.find((entry) => entry.id === id);
	if (!finished) return;
	finished.phase = 'done';
	finished.verdict = 'clean' as ScanVerdict;

	const stillOnVpn = app.downloads.some(
		(item) => item.route === 'vpn' && item.phase !== 'done' && item.phase !== 'error'
	);
	if (!stillOnVpn) removeBanner('vpn_route');
}

export function removeDownload(id: string) {
	app.downloads = app.downloads.filter((item) => item.id !== id);
	if (!app.downloads.some((item) => item.route === 'vpn')) removeBanner('vpn_route');
}

export function clearDownloads() {
	app.downloads = [];
	removeBanner('vpn_route');
}

export async function enable() {
	if (app.state !== 'OFF') return;
	const my = ++generation;
	app.failureCode = null;
	app.banners = app.banners.filter((b) => b !== 'failclosed');
	app.watchdog = 'ok';

	setState('PREFLIGHT', 6);
	await sleep(900);
	if (my !== generation) return;

	setState('ARMING', 24);
	app.firewallArmed = true;
	app.macRandomized = true;
	await sleep(1000);
	if (my !== generation) return;

	if (app.demoPortal) {
		setState('PORTAL', 34);
		addBanner('portal_open');
		await sleep(3600);
		if (my !== generation) return;
		removeBanner('portal_open');
	}

	app.vpnConnected = true;
	app.country = app.settings.vpnCountry === 'random' ? pickCountry() : app.settings.vpnCountry;
	setState('VPN_UP', 55);
	await sleep(1000);
	if (my !== generation) return;

	setState('TOR_UP', 62);
	for (const p of [68, 76, 88, 96, 100]) {
		await sleep(320);
		if (my !== generation) return;
		app.torBootstrap = p;
		app.progress = 62 + Math.round((p / 100) * 36);
	}
	app.circuit = true;
	await sleep(350);
	if (my !== generation) return;

	setState('READY', 100);
	showToast('Schutz steht. Nur Tor und Freigaben kommen raus.');
}

export async function disable() {
	if (app.state === 'OFF' || app.state === 'TEARDOWN') return;
	const my = ++generation;
	const startedAt = app.since;

	setState('TEARDOWN', 40);
	await sleep(600);
	if (my !== generation) return;

	setState('TEARDOWN', 72);
	await sleep(650);
	if (my !== generation) return;

	const counts = summarizeDownloads();
	app.lastSummary = {
		startedAt,
		endedAt: Date.now(),
		stateReached: 'READY',
		clean: counts.clean,
		warned: counts.warned,
		quarantined: counts.quarantined,
		failed: counts.failed,
		failclosedCount: 0,
		reasons: []
	};
	app.summaryOpen = true;

	app.firewallArmed = false;
	app.vpnConnected = false;
	app.country = null;
	app.torBootstrap = 0;
	app.circuit = false;
	app.macRandomized = app.settings.macMode === 'original' ? false : true;
	app.banners = [];
	setState('OFF', 0);
}

export async function panic() {
	const my = ++generation;
	removeBanner('portal_open');
	removeBanner('fallback_active');
	setState('TEARDOWN', 55);
	app.downloads = [];
	showToast('Panik: Apps geschlossen, Netz getrennt.');
	await sleep(550);
	if (my !== generation) return;
	app.firewallArmed = false;
	app.vpnConnected = false;
	app.country = null;
	app.torBootstrap = 0;
	app.circuit = false;
	app.banners = [];
	setState('OFF', 0);
}

export async function recover() {
	const my = ++generation;
	setState('RECOVERY', 40);
	await sleep(700);
	if (my !== generation) return;
	app.progress = 80;
	await sleep(700);
	if (my !== generation) return;
	app.firewallArmed = false;
	app.vpnConnected = false;
	app.country = null;
	app.torBootstrap = 0;
	app.circuit = false;
	app.failureCode = null;
	app.watchdog = 'ok';
	removeBanner('failclosed');
	removeBanner('degraded');
	setState('OFF', 0);
	showToast('Netz wiederhergestellt. Es ist kein Filter mehr aktiv.');
}

/** Demo helper: jump straight to a state with consistent details. */
export function jumpTo(state: AppState) {
	generation++;
	const armed = ['ARMING', 'PORTAL', 'VPN_UP', 'TOR_UP', 'READY', 'DEGRADED', 'FAILCLOSED'].includes(state);

	app.firewallArmed = armed;
	app.banners = [];
	app.failureCode = null;
	app.watchdog = 'ok';

	switch (state) {
		case 'OFF':
			app.vpnConnected = false;
			app.country = null;
			app.torBootstrap = 0;
			app.circuit = false;
			app.macRandomized = false;
			break;
		case 'PREFLIGHT':
			app.vpnConnected = false;
			app.country = null;
			app.torBootstrap = 0;
			app.circuit = false;
			break;
		case 'ARMING':
			app.vpnConnected = false;
			app.country = null;
			app.torBootstrap = 0;
			app.circuit = false;
			app.macRandomized = true;
			break;
		case 'PORTAL':
			app.vpnConnected = false;
			app.country = null;
			app.torBootstrap = 0;
			app.circuit = false;
			app.macRandomized = true;
			addBanner('portal_open');
			break;
		case 'VPN_UP':
			app.vpnConnected = true;
			app.country = 'se';
			app.torBootstrap = 0;
			app.circuit = false;
			app.macRandomized = true;
			break;
		case 'TOR_UP':
			app.vpnConnected = true;
			app.country = 'se';
			app.torBootstrap = 72;
			app.circuit = false;
			app.macRandomized = true;
			break;
		case 'READY':
			app.vpnConnected = true;
			app.country = 'se';
			app.torBootstrap = 100;
			app.circuit = true;
			app.macRandomized = true;
			break;
		case 'DEGRADED':
			app.vpnConnected = true;
			app.country = 'nl';
			app.torBootstrap = 100;
			app.circuit = true;
			app.macRandomized = true;
			app.watchdog = 'warn';
			app.failureCode = 'E_TOR_CIRCUIT_FAILED';
			addBanner('degraded');
			break;
		case 'FAILCLOSED':
			app.vpnConnected = false;
			app.country = null;
			app.torBootstrap = 0;
			app.circuit = false;
			app.macRandomized = true;
			app.watchdog = 'critical';
			app.failureCode = 'E_TOR_BOOTSTRAP_TIMEOUT';
			addBanner('failclosed');
			break;
		case 'TEARDOWN':
			app.vpnConnected = true;
			app.country = 'se';
			app.torBootstrap = 40;
			app.circuit = false;
			break;
		case 'RECOVERY':
			app.vpnConnected = false;
			app.country = null;
			app.torBootstrap = 0;
			app.circuit = false;
			break;
	}

	setState(state);
}

export function updateSetting<K extends keyof Settings>(key: K, value: Settings[K]) {
	app.settings[key] = value;
	if (key === 'macMode' && value === 'original') app.macRandomized = false;
}
