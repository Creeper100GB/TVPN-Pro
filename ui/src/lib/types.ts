export type AppState =
	| 'OFF'
	| 'PREFLIGHT'
	| 'ARMING'
	| 'PORTAL'
	| 'VPN_UP'
	| 'TOR_UP'
	| 'READY'
	| 'DEGRADED'
	| 'FAILCLOSED'
	| 'TEARDOWN'
	| 'RECOVERY';

export type Tone = 'neutral' | 'busy' | 'ok' | 'warn' | 'danger';

export type BannerKind =
	| 'portal_open'
	| 'fallback_active'
	| 'degraded'
	| 'failclosed'
	| 'mac_unsupported'
	| 'hostname_unchanged';

export type WatchdogLevel = 'ok' | 'warn' | 'critical';

export type BlockFallback = 'off' | 'suggest' | 'auto';
export type MacMode = 'random' | 'original';

export type DownloadPhase = 'queued' | 'active' | 'paused' | 'done' | 'error';
export type ScanVerdict = 'clean' | 'warned' | 'quarantined' | 'failed';

export interface DownloadItem {
	id: string;
	label: string;
	phase: DownloadPhase;
	progress: number;
	verdict: ScanVerdict | null;
}

export interface Settings {
	vpnEnabled: boolean;
	vpnCountry: string;
	torBridges: boolean;
	blockFallback: BlockFallback;
	macMode: MacMode;
	torrentBlock: boolean;
	clipboardWatch: boolean;
	scannerHashLookup: boolean;
	portalTimeoutSec: number;
}

export interface SessionSummary {
	startedAt: number;
	endedAt: number;
	stateReached: AppState;
	clean: number;
	warned: number;
	quarantined: number;
	failed: number;
	failclosedCount: number;
	reasons: string[];
}

export const STATE_ORDER: AppState[] = [
	'OFF',
	'PREFLIGHT',
	'ARMING',
	'PORTAL',
	'VPN_UP',
	'TOR_UP',
	'READY',
	'DEGRADED',
	'FAILCLOSED',
	'TEARDOWN',
	'RECOVERY'
];
