import { COUNTRIES } from './countries';
import { STATE_META } from './states';
import type {
	AppState,
	BannerKind,
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
	downloads: { active: number; quarantined: number };
	settings: Settings;
	demoPortal: boolean;
	failureCode: string | null;
	lastSummary: SessionSummary | null;
	summaryOpen: boolean;
	settingsOpen: boolean;
	toast: string | null;
}

const defaultSettings: Settings = {
	vpnEnabled: true,
	vpnCountry: 'random',
	torBridges: false,
	blockFallback: 'auto',
	macMode: 'random',
	torrentBlock: true,
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
	downloads: { active: 0, quarantined: 0 },
	settings: { ...defaultSettings },
	demoPortal: false,
	failureCode: null,
	lastSummary: null,
	summaryOpen: false,
	settingsOpen: false,
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
	app.downloads.active = 0;
	showToast('Schutz steht. Nur Tor und Freigaben kommen raus.');
}

export async function disable() {
	if (app.state === 'OFF' || app.state === 'TEARDOWN') return;
	const my = ++generation;
	const startedAt = app.since;

	setState('TEARDOWN', 40);
	await sleep(600);
	if (my !== generation) return;

	app.downloads.active = 0;
	setState('TEARDOWN', 72);
	await sleep(650);
	if (my !== generation) return;

	app.lastSummary = {
		startedAt,
		endedAt: Date.now(),
		stateReached: 'READY',
		clean: 3,
		warned: 1,
		quarantined: 0,
		failed: 0,
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
	app.downloads.active = 0;
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
