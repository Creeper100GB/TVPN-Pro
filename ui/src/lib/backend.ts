// Bridge to the Tauri shell. In the browser preview every call is a no-op and
// the mock store takes over, so one component tree serves both.

import { isDesktop } from './desktop';
import type { DownloadRoute } from './types';

export interface DownloadRequest {
	id: string;
	url: string;
	route: DownloadRoute;
}

export interface DownloadProgress {
	id: string;
	received: number;
	total: number | null;
}

export interface DownloadDone {
	id: string;
	path: string;
	bytes: number;
	sha256: string;
	verdict: 'clean' | 'threat' | 'unknown';
}

export interface DownloadFailure {
	id: string;
	code: string;
}

export async function startDownload(request: DownloadRequest): Promise<void> {
	if (!isDesktop) return;
	const { invoke } = await import('@tauri-apps/api/core');
	await invoke('download_start', {
		id: request.id,
		url: request.url,
		route: request.route
	});
}

export async function cancelDownload(id: string): Promise<void> {
	if (!isDesktop) return;
	const { invoke } = await import('@tauri-apps/api/core');
	await invoke('download_cancel', { id });
}

export async function watchDownloads(handlers: {
	progress: (event: DownloadProgress) => void;
	done: (event: DownloadDone) => void;
	error: (event: DownloadFailure) => void;
}): Promise<() => void> {
	if (!isDesktop) return () => {};

	const { listen } = await import('@tauri-apps/api/event');
	const stops = await Promise.all([
		listen<DownloadProgress>('download-progress', (event) => handlers.progress(event.payload)),
		listen<DownloadDone>('download-done', (event) => handlers.done(event.payload)),
		listen<DownloadFailure>('download-error', (event) => handlers.error(event.payload))
	]);

	return () => {
		for (const stop of stops) stop();
	};
}

/** Turns backend codes into something a person can act on. */
export function explainDownloadError(code: string): string {
	const messages: Record<string, string> = {
		vpn_requires_service: 'Die VPN-Route braucht den Dienst. Der kommt in Phase 2.',
		route: 'Diese Route ist nicht verfügbar.',
		network: 'Netzwerkfehler. Läuft der Tor-Proxy auf 127.0.0.1:9150?',
		proxy: 'Tor-Proxy konnte nicht gesetzt werden.',
		cancelled: 'Abgebrochen.',
		no_download_dir: 'Der Download-Ordner wurde nicht gefunden.',
		create_file: 'Die Datei konnte nicht angelegt werden.',
		read: 'Der Empfang ist abgebrochen.',
		write: 'Schreiben auf die Platte ist fehlgeschlagen.',
		flush: 'Die Datei konnte nicht abgeschlossen werden.'
	};

	if (code.startsWith('http_')) {
		return `Der Server antwortet mit ${code.slice(5)}.`;
	}

	return messages[code] ?? `Fehler: ${code}`;
}
