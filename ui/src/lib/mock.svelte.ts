import { cancelAllDownloads, serviceCommand, serviceStatus } from './backend';
import { COUNTRIES } from './countries';
import { isDesktop } from './desktop';
import { isArmed, isTransient, STATE_META } from './states';
import { summarizeDownloadList } from './summary';
import { shouldWriteSummary, teardownStart } from './teardown';
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
	serviceReachable: boolean;
	serviceEverReachable: boolean;
	watchdog: WatchdogLevel;
	banners: BannerKind[];
	downloads: DownloadItem[];
	settings: Settings;
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
	serviceReachable: true,
	serviceEverReachable: false,
	watchdog: 'ok',
	banners: [],
	downloads: [],
	settings: { ...defaultSettings },
	failureCode: null,
	lastSummary: null,
	summaryOpen: false,
	settingsOpen: false,
	downloadsOpen: false,
	toast: null
});

let generation = 0;
let toastTimer: ReturnType<typeof setTimeout> | null = null;

/** When the current protection session started, not when READY was reached. */
let sessionStartedAt: number | null = null;

function sleep(ms: number) {
	return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

/** Panic teardown in the browser preview. Long enough that the E2E can observe it. */
const teardownDuration = 2000;

let pollTimer: ReturnType<typeof setInterval> | null = null;
let pollBusy = false;
let settingsLoaded = false;

function applySettings(raw: Record<string, unknown>) {
	app.settings = {
		vpnEnabled: Boolean(raw.vpn_enabled),
		// The service stores ISO codes in upper case, the picker uses lower case.
		vpnCountry: String(raw.vpn_country ?? 'random').toLowerCase(),
		torBridges: Boolean(raw.tor_bridges),
		blockFallback: (raw.block_fallback as Settings['blockFallback']) ?? 'auto',
		macMode: (raw.mac_mode as Settings['macMode']) ?? 'random',
		torrentMode: (raw.torrent_mode as Settings['torrentMode']) ?? 'blocked',
		clipboardWatch: Boolean(raw.clipboard_watch),
		scannerHashLookup: Boolean(raw.scanner_hash_lookup),
		portalTimeoutSec: Number(raw.portal_timeout_sec ?? 180)
	};
}

function applyServiceStatus(data: Record<string, unknown>) {
	const state = data.state as AppState;
	if (!STATE_META[state]) return;

	app.state = state;
	app.since = Date.parse(String(data.since)) || Date.now();
	const vpn = (data.vpn ?? {}) as Record<string, unknown>;
	app.vpnConnected = vpn.state === 'connected';
	app.country = typeof vpn.country === 'string' ? vpn.country : null;
	const tor = (data.tor ?? {}) as Record<string, unknown>;
	app.torBootstrap = Number(tor.bootstrap ?? 0);
	app.circuit = Boolean(tor.circuit);
	const watchdog = (data.watchdog ?? {}) as Record<string, unknown>;
	app.watchdog = (watchdog.level as WatchdogLevel) ?? 'ok';
	app.banners = Array.isArray(data.banners) ? (data.banners as BannerKind[]) : [];
	const shield = (data.shield ?? {}) as Record<string, unknown>;
	app.firewallArmed =
		typeof shield.firewall_armed === 'boolean'
			? Boolean(shield.firewall_armed)
			: isArmed(state);
	app.macRandomized = Boolean(shield.mac_randomized);
	if (state === 'FAILCLOSED' && !app.firewallArmed && !app.banners.includes('no_filter')) {
		app.banners = [...app.banners, 'no_filter'];
	}
	app.progress = STATE_META[state].progress;
	// A teardown keeps its code: only a genuinely failed teardown lands in
	// TEARDOWN with one, and the view offers the reset instead of a busy state.
	if (state !== 'FAILCLOSED' && state !== 'TEARDOWN') app.failureCode = null;
}

function markServiceLost() {
	app.serviceReachable = false;
	if (!app.banners.includes('service_unavailable')) {
		app.banners = [...app.banners, 'service_unavailable'];
	}
}

/** Pulls one status snapshot and the settings on first contact. */
export async function refreshService(): Promise<void> {
	if (!isDesktop) return;
	const data = await serviceStatus();
	app.serviceReachable = true;
	app.serviceEverReachable = true;
	applyServiceStatus(data);
	if (!settingsLoaded) {
		const settings = await serviceCommand('get_settings');
		applySettings(settings);
		settingsLoaded = true;
	}
}

function startPolling() {
	if (!isDesktop || pollTimer) return;
	pollTimer = setInterval(async () => {
		if (pollBusy || isTransient(app.state)) return;
		pollBusy = true;
		try {
			await refreshService();
		} catch {
			// A lost service is not OFF. The filters may still be armed, and the
			// UI must not keep claiming "protected" or "off" without evidence.
			app.serviceReachable = false;
			if (!app.banners.includes('service_unavailable')) {
				app.banners = [...app.banners, 'service_unavailable'];
			}
		} finally {
			pollBusy = false;
		}
	}, 2500);
}

if (isDesktop) {
	setTimeout(() => {
		void refreshService().catch(() => {});
	}, 100);
	startPolling();
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

/** Maschinen-Codes in eine Handlung übersetzen, ohne Details zu erfinden. */
export function explainFailure(code: string): string {
	const messages: Record<string, string> = {
		E_PREFLIGHT_MISSING_COMPONENT:
			'Ein Baustein fehlt. Prüfe die Installation von Tor bzw. Mullvad oder schalte VPN in den Einstellungen aus.',
		E_PREFLIGHT_VPN_NOT_LOGGED_IN:
			'Mullvad ist nicht angemeldet. Öffne die Mullvad-App oder nutze den Tor-only-Modus.',
		E_PREFLIGHT_NO_ADMIN: 'Der Dienst hat nicht die nötigen Rechte. Ist er installiert und gestartet?',
		E_FIREWALL_ARM_FAILED: 'Die Firewall konnte nicht gesetzt werden. Das Netz bleibt gesperrt.',
		E_FIREWALL_VERIFY_MISMATCH: 'Der Filtersatz ist unvollständig. Notfall-Reset starten.',
		E_SETTINGS_PERSIST: 'Die Einstellung wurde nicht gespeichert. Prüfe die Rechte von %ProgramData%\\TVPN.',
		E_TOR_BOOTSTRAP_TIMEOUT: 'Tor erreicht keinen Circuit. Netzwerk und Tor-Dateien prüfen.',
		E_TOR_CONTROL_AUTH: 'Tor-Control-Authentisierung fehlgeschlagen. Tor neu starten.',
		E_TOR_CIRCUIT_FAILED: 'Tor hat keinen Circuit. Netzwerk prüfen.',
		E_VPN_CONNECT_TIMEOUT: 'Die VPN-Verbindung kam nicht zustande.',
		E_WATCHDOG_CRITICAL: 'Der Wächter hat eine Komponente verloren. Das Netz ist gesperrt.',
		service_unavailable: 'Der Dienst ist nicht installiert oder läuft nicht. Installiere TVPN Pro erneut oder starte den Dienst.',
		service_token: 'Der Dienst läuft, aber die UI kann das Sitzungs-Token nicht lesen.'
	};
	return messages[code] ?? 'Unbekannter Fehler. Details stehen im Code.';
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

/** One session summary from the service answer plus the local download counts. */
function buildSummary(
	serviceSummary: Record<string, unknown> | null,
	startedAt: number,
	endedAt: number,
	reachedLocally: AppState
): SessionSummary {
	const counts = summarizeDownloads();
	const source = serviceSummary ?? {};
	return {
		startedAt,
		endedAt,
		stateReached: (source.state_reached as AppState) ?? reachedLocally,
		clean: counts.clean,
		warned: counts.warned,
		quarantined: counts.quarantined,
		failed: counts.failed,
		failclosedCount: Number(source.failclosed_count ?? 0),
		reasons: Array.isArray(source.reasons) ? source.reasons.map(String) : []
	};
}

function summarizeDownloads() {
	return summarizeDownloadList(app.downloads);
}

/**
 * Reads the service summary and falls back to the local counts.
 *
 * Kept apart from the teardown call: a missing summary is a reporting gap, not a
 * failure to end protection, so it must never surface as FAILCLOSED.
 */
async function sessionSummary(startedAt: number, reached: AppState): Promise<SessionSummary> {
	try {
		const raw = await serviceCommand('get_session_summary');
		return buildSummary(
			raw && typeof raw === 'object' ? raw : null,
			startedAt,
			Date.now(),
			reached
		);
	} catch {
		return buildSummary(null, startedAt, Date.now(), reached);
	}
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
		return {
			ok: false,
			error:
				'Torrents sind nicht implementiert. Es läuft kein Torrent-Client, eine .torrent-Datei wird wie jede andere Datei gespeichert, aber nicht geöffnet oder geseedet.',
			label: '',
			source: 'torrent'
		};
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
	const last = parsed.pathname.split('/').filter(Boolean).pop() ?? 'download.bin';
	try {
		return decodeURIComponent(last);
	} catch {
		return last;
	}
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
	if (isDesktop) void cancelAllDownloads().catch(() => {});
	app.downloads = [];
	removeBanner('vpn_route');
}

export async function enable() {
	if (app.state !== 'OFF') return;

	// The session starts here, not when READY is reached: PREFLIGHT, ARMING and
	// the Tor bootstrap are part of the time the user was protected.
	sessionStartedAt = Date.now();

	if (isDesktop) {
		app.failureCode = null;
		app.banners = app.banners.filter((b) => b !== 'failclosed');
		app.watchdog = 'ok';
		setState('PREFLIGHT', 6);
		try {
			await serviceCommand('enable', { net_mode: 'focus', protect_mode: 'tor_vpn' });
			await refreshService();
			const reached = app.state as AppState;
			if (reached === 'READY') {
				showToast('Tor läuft. Die VPN-Kette ist nicht unabhängig geprüft.');
			} else if (reached === 'DEGRADED') {
				showToast(
					app.settings.vpnEnabled
						? 'Eine Komponente ist eingeschränkt. Prüfe den Status.'
						: 'Tor läuft. Die VPN-Kette ist aus; der ISP sieht Tor-Traffic.'
				);
			}
		} catch (cause) {
			const code = String(cause);
			app.failureCode = code;
			if (
				code === 'service_unavailable' ||
				code === 'service_task' ||
				code === 'service_lock'
			) {
				markServiceLost();
				if (app.serviceEverReachable) {
					setState('FAILCLOSED', 100);
					showToast('Der Dienst ist nicht erreichbar. Der Firewall-Status ist unbekannt.');
				} else {
					setState('OFF', 0);
					showToast('Der Dienst ist nicht installiert oder läuft nicht.');
				}
			} else {
				app.banners = [...app.banners.filter((b) => b !== 'failclosed'), 'failclosed'];
				setState('FAILCLOSED', 100);
				await refreshService().catch(() => {});
			}
		}
		return;
	}

	const my = ++generation;
	app.failureCode = null;
	app.banners = app.banners.filter((b) => b !== 'failclosed');
	app.watchdog = 'ok';

	setState('PREFLIGHT', 6);
	await sleep(900);
	if (my !== generation) return;

	setState('ARMING', 24);
	app.firewallArmed = true;
	// A random MAC is a setting, not a side effect of switching on.
	app.macRandomized = app.settings.macMode !== 'original';
	await sleep(1000);
	if (my !== generation) return;

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
	showToast('Tor läuft. Die VPN-Kette ist nicht unabhängig geprüft.');
}

export async function disable() {
	if (app.state === 'OFF' || app.state === 'TEARDOWN') return;

	if (isDesktop) {
		// Capture the session start before TEARDOWN resets `since`, otherwise the
		// summary duration is only the length of the teardown.
		const startedAt = sessionStartedAt ?? app.since;
		const reached = app.state;
		const start = teardownStart(app.banners);
		app.failureCode = start.failureCode;
		app.banners = start.banners;
		setState(start.state, 40);
		try {
			await cancelAllDownloads().catch(() => {});
			await serviceCommand('disable', { downloads_policy: 'wait' });
			app.lastSummary = await sessionSummary(startedAt, reached);
			app.summaryOpen = true;
			sessionStartedAt = null;
			await refreshService();
			showToast('Schutz beendet.');
		} catch (cause) {
			app.failureCode = String(cause);
			if (String(cause) === 'service_unavailable') markServiceLost();
			setState('FAILCLOSED', 100);
			await refreshService().catch(() => {});
		}
		return;
	}

	const my = ++generation;
	const startedAt = sessionStartedAt ?? app.since;
	const reached = app.state;

	const start = teardownStart(app.banners);
	app.failureCode = start.failureCode;
	app.banners = start.banners;
	setState(start.state, 40);
	await sleep(600);
	if (my !== generation) return;

	setState('TEARDOWN', 72);
	await sleep(650);
	if (my !== generation) return;

	app.lastSummary = buildSummary(null, startedAt, Date.now(), reached);
	app.summaryOpen = true;
	sessionStartedAt = null;

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
	if (isDesktop) {
		const startedAt = sessionStartedAt ?? app.since;
		const reached = app.state;
		removeBanner('portal_open');
		removeBanner('fallback_active');
		const start = teardownStart(app.banners);
		app.failureCode = start.failureCode;
		app.banners = start.banners;
		setState(start.state, 55);
		try {
			await cancelAllDownloads().catch(() => {});
			await serviceCommand('panic');
			// A panic from OFF has no session to report, and a second panic during a
			// hanging teardown must not report TEARDOWN as a reached state. The
			// download list is cleared either way; only the report is gated.
			if (shouldWriteSummary(reached)) {
				// Build the summary before the list is cleared: the counts come from it.
				app.lastSummary = await sessionSummary(startedAt, reached);
				app.summaryOpen = true;
			}
			sessionStartedAt = null;
			app.downloads = [];
			await refreshService();
			showToast('Panik: Schutz beendet, Netz wieder offen.');
		} catch (cause) {
			app.failureCode = String(cause);
			if (String(cause) === 'service_unavailable') markServiceLost();
			setState('FAILCLOSED', 100);
		}
		return;
	}

	const my = ++generation;
	const startedAt = sessionStartedAt ?? app.since;
	const reached = app.state;
	removeBanner('portal_open');
	removeBanner('fallback_active');
	const start = teardownStart(app.banners);
	app.failureCode = start.failureCode;
	app.banners = start.banners;
	setState(start.state, 55);
	await sleep(teardownDuration);
	if (my !== generation) return;
	if (shouldWriteSummary(reached)) {
		app.lastSummary = buildSummary(null, startedAt, Date.now(), reached);
		app.summaryOpen = true;
	}
	sessionStartedAt = null;
	app.downloads = [];
	showToast('Panik: Schutz beendet, Netz offen.');
	app.firewallArmed = false;
	app.vpnConnected = false;
	app.country = null;
	app.torBootstrap = 0;
	app.circuit = false;
	app.banners = [];
	setState('OFF', 0);
}

export async function recover() {
	if (isDesktop) {
		setState('RECOVERY', 40);
		try {
			await serviceCommand('recover');
			await refreshService();
			showToast('Netz wiederhergestellt. Es ist kein Filter mehr aktiv.');
		} catch (cause) {
			app.failureCode = String(cause);
			if (String(cause) === 'service_unavailable') markServiceLost();
			setState('FAILCLOSED', 100);
		}
		return;
	}

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

export function updateSetting<K extends keyof Settings>(key: K, value: Settings[K]) {
	app.settings[key] = value;
	if (key === 'macMode' && value === 'original') app.macRandomized = false;

	if (!isDesktop) return;

	const keys: Record<keyof Settings, string> = {
		vpnEnabled: 'vpn_enabled',
		vpnCountry: 'vpn_country',
		torBridges: 'tor_bridges',
		blockFallback: 'block_fallback',
		macMode: 'mac_mode',
		torrentMode: 'torrent_mode',
		clipboardWatch: 'clipboard_watch',
		scannerHashLookup: 'scanner_hash_lookup',
		portalTimeoutSec: 'portal_timeout_sec'
	};

	// The service validates upper-case ISO codes and a numeric timeout.
	let payload: unknown = value;
	if (key === 'vpnCountry') {
		const country = String(value).toLowerCase();
		payload = country === 'random' ? 'random' : country.toUpperCase();
	} else if (key === 'portalTimeoutSec') {
		payload = Number(value);
	}

	void serviceCommand('set_settings', { settings: { [keys[key]]: payload } })
		.then(() => showToast('Einstellung gespeichert.'))
		.catch((cause) => {
			showToast('Nicht gespeichert: ' + String(cause));
			settingsLoaded = false;
			void refreshService().catch(() => {});
		});
}
