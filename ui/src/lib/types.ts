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
	| 'hostname_unchanged'
	| 'vpn_route';

export type WatchdogLevel = 'ok' | 'warn' | 'critical';

export type BlockFallback = 'off' | 'suggest' | 'auto';
export type MacMode = 'random' | 'original';
export type TorrentMode = 'blocked' | 'vpn_only';

export type DownloadRoute = 'tor' | 'direct' | 'vpn';
export type DownloadSource = 'https' | 'torrent';

export type DownloadPhase = 'queued' | 'active' | 'paused' | 'done' | 'error';
export type ScanVerdict =
	| 'clean'
	| 'warned'
	| 'quarantined'
	| 'failed'
	| 'threat'
	| 'unknown';

export interface DownloadItem {
	id: string;
	label: string;
	phase: DownloadPhase;
	progress: number;
	verdict: ScanVerdict | null;
	route: DownloadRoute;
	source: DownloadSource;
}

export interface Settings {
	vpnEnabled: boolean;
	vpnCountry: string;
	torBridges: boolean;
	blockFallback: BlockFallback;
	macMode: MacMode;
	torrentMode: TorrentMode;
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
