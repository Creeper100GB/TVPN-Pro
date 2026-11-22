<script lang="ts">
	import { onMount } from 'svelte';
	import {
		Activity,
		Download,
		Globe,
		Network,
		Settings as SettingsIcon,
		ShieldAlert,
		ShieldCheck,
		Wifi,
		Zap
	} from 'lucide-svelte';
	import { countryName } from '../lib/countries';
	import { watchDownloads } from '../lib/backend';
	import { isDesktop, windowAction } from '../lib/desktop';
	import {
		activeDownloadCount,
		app,
		disable,
		enable,
		panic,
		patchDownload,
		quarantinedDownloadCount,
		recover,
		showToast
	} from '../lib/mock.svelte';
	import { STATE_META, TONE_VAR, isArmed, isTransient } from '../lib/states';
	import Banner from '../lib/components/Banner.svelte';
	import DownloadSheet from '../lib/components/DownloadSheet.svelte';
	import PowerButton from '../lib/components/PowerButton.svelte';
	import SessionSummary from '../lib/components/SessionSummary.svelte';
	import SettingsSheet from '../lib/components/SettingsSheet.svelte';
	import StatRow from '../lib/components/StatRow.svelte';
	import { APP_VERSION } from '../lib/version';

	const meta = $derived(STATE_META[app.state]);
	const noFilter = $derived(app.state === 'FAILCLOSED' && !app.firewallArmed);
	const stateLabel = $derived(noFilter ? 'Vorprüfung fehlgeschlagen' : meta.label);
	const stateHint = $derived(
		noFilter
			? 'Es ist kein Filter aktiv. Das Netz ist offen. Prüfe Tor, Mullvad und die Einstellungen.'
			: meta.hint
	);
	const busy = $derived(isTransient(app.state));
	const tone = $derived(meta.tone);
	const primaryDisabled = $derived(busy);

	const startLabel = $derived(
		new Date(app.since).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })
	);

	const vpnValue = $derived(
		app.vpnConnected ? countryName(app.country) : app.settings.vpnEnabled ? 'Nicht verbunden' : 'Aus'
	);
	const vpnTone = $derived(app.vpnConnected ? 'ok' : 'neutral');

	const torValue = $derived(
		app.circuit ? 'Circuit steht' : app.torBootstrap > 0 ? `Bootstrap ${app.torBootstrap} %` : 'Aus'
	);
	const torTone = $derived(app.circuit ? 'ok' : app.torBootstrap > 0 ? 'warn' : 'neutral');

	const firewallValue = $derived(app.firewallArmed ? 'Allowlist aktiv' : 'Aus');
	const firewallTone = $derived(app.firewallArmed ? 'ok' : app.state === 'FAILCLOSED' ? 'danger' : 'neutral');

	const watchdogValue = $derived(
		app.watchdog === 'ok' ? 'OK' : app.watchdog === 'warn' ? 'Warnung' : 'Kritisch'
	);

	const powerLabel = $derived(
		app.state === 'OFF'
			? 'Schutz einschalten'
			: app.state === 'FAILCLOSED'
				? 'Notfall-Reset starten'
				: app.state === 'READY' || app.state === 'DEGRADED'
					? 'Schutz ausschalten'
					: meta.action
	);

	onMount(() => {
		let disposed = false;
		let stop: (() => void) | undefined;

		void (async () => {
			try {
				const unsubscribe = await watchDownloads({
					progress: (event) => {
						const percent = event.total
							? Math.min(99, Math.round((event.received / event.total) * 100))
							: 50;
						patchDownload(event.id, { phase: 'active', progress: percent });
					},
					done: (event) => {
						patchDownload(event.id, {
							phase: 'done',
							progress: 100,
							verdict:
								event.verdict === 'clean'
									? 'clean'
									: event.verdict === 'threat'
										? 'threat'
										: 'unknown'
						});
						showToast(
							event.verdict === 'clean'
								? 'Download fertig, Scan sauber.'
								: 'Download fertig. Urteil prüfen.'
						);
					},
					error: (event) => {
						console.error('download-error', event.code);
						patchDownload(event.id, { phase: 'error' });
					}
				});
				if (disposed) unsubscribe();
				else stop = unsubscribe;
			} catch {
				// Browser preview or missing permission: mock mode stays usable.
			}
		})();

		return () => {
			disposed = true;
			stop?.();
		};
	});

	function primary() {
		switch (app.state) {
			case 'OFF':
				void enable();
				break;
			case 'FAILCLOSED':
				void recover();
				break;
			case 'READY':
			case 'DEGRADED':
				void disable();
				break;
			case 'PORTAL':
				showToast('VPN-Browser geöffnet. Das Portal sieht dein Gerät ohne VPN.');
				break;
			default:
				break;
		}
	}
</script>

<div class="stage">
	<div class="window">
		<header class="titlebar" data-tauri-drag-region>
			<div class="titlebar-left" data-tauri-drag-region>
				{#if isDesktop}
					<div class="window-controls">
						<button
							type="button"
							class="traffic close"
							aria-label="Fenster schließen"
							onclick={() => windowAction('close')}
						></button>
						<button
							type="button"
							class="traffic minimize"
							aria-label="Fenster minimieren"
							onclick={() => windowAction('minimize')}
						></button>
						<button
							type="button"
							class="traffic maximize"
							aria-label="Fenster maximieren"
							onclick={() => windowAction('maximize')}
						></button>
					</div>
				{/if}

				<div class="brand" data-tauri-drag-region>
					<div class="logo"><ShieldCheck size={19} strokeWidth={2.1} /></div>
					<div data-tauri-drag-region>
						<h1 data-tauri-drag-region>TVPN Pro</h1>
						<p class="sub" data-tauri-drag-region>Fokus-Modus · keine Telemetrie</p>
					</div>
				</div>
			</div>

			<div class="actions">
				<button
					type="button"
					class="icon-button"
					aria-label="Einstellungen öffnen"
					onclick={() => (app.settingsOpen = true)}
				>
					<SettingsIcon size={17} />
				</button>
			</div>
		</header>

		<div class="alpha" role="note">
			<ShieldAlert size={13} />
			Alpha: Der Dienst setzt Firewall und Tor. Ohne Mullvad-Konto bleibt die VPN-Kette ungeprüft.
		</div>

		<div class="content">
			<section class="hero card" aria-labelledby="state-title">
				<p class="eyebrow">Status</p>

				<PowerButton
					state={app.state}
					progress={app.progress}
					{tone}
					label={powerLabel}
					disabled={primaryDisabled}
					onclick={primary}
				/>

				<div class="headline">
					<h2 id="state-title" aria-live="polite">{stateLabel}</h2>
					<p class="hint">{stateHint}</p>
				</div>

				{#if app.banners.length > 0}
					<div class="banners">
						{#each app.banners as kind (kind)}
							<Banner {kind} />
						{/each}
					</div>
				{/if}

				{#if app.failureCode && app.state === 'FAILCLOSED'}
					<p class="error-code">Code: {app.failureCode}</p>
				{/if}
			</section>

			<aside class="side">
				<section class="card" aria-labelledby="conn-title">
					<header class="card-head">
						<h3 id="conn-title">Verbindung</h3>
						<span class="chip">{isArmed(app.state) ? 'Scharf' : 'Bereit'}</span>
					</header>
					<div class="card-body">
						<StatRow label="VPN" value={vpnValue} tone={vpnTone} divider={false}>
							{#snippet icon()}<Wifi size={15} strokeWidth={1.8} />{/snippet}
						</StatRow>
						<StatRow label="Tor" value={torValue} tone={torTone}>
							{#snippet icon()}<Globe size={15} strokeWidth={1.8} />{/snippet}
						</StatRow>
						<StatRow label="Firewall" value={firewallValue} tone={firewallTone}>
							{#snippet icon()}<Network size={15} strokeWidth={1.8} />{/snippet}
						</StatRow>
						<StatRow
							label="Wächter"
							value={watchdogValue}
							tone={app.watchdog === 'ok' ? 'ok' : app.watchdog === 'warn' ? 'warn' : 'danger'}
						>
							{#snippet icon()}<Activity size={15} strokeWidth={1.8} />{/snippet}
						</StatRow>
					</div>
				</section>

				<section class="card" aria-labelledby="dl-title">
					<header class="card-head">
						<h3 id="dl-title">Downloads</h3>
						<button
							type="button"
							class="chip-button"
							aria-label="Downloads öffnen"
							onclick={() => (app.downloadsOpen = true)}
						>
							Öffnen
						</button>
					</header>
					<div class="card-body">
						<StatRow
							label="Aktiv"
							value={String(activeDownloadCount())}
							tone={activeDownloadCount() > 0 ? 'ok' : 'neutral'}
							divider={false}
						>
							{#snippet icon()}<Download size={15} strokeWidth={1.8} />{/snippet}
						</StatRow>
						<StatRow
							label="In Quarantäne"
							value={String(quarantinedDownloadCount())}
							tone={quarantinedDownloadCount() > 0 ? 'warn' : 'neutral'}
						>
							{#snippet icon()}<ShieldAlert size={15} strokeWidth={1.8} />{/snippet}
						</StatRow>
						<p class="small-note">
							Scans laufen über Defender und ClamAV. Keine 0-Days, keine Dokumente, die nach dem
							Ausschalten die echte IP verraten.
						</p>
					</div>
				</section>

				<p class="fineprint">
					Reduziert Spuren. Ersetzt kein vorsichtiges Verhalten und verspricht keine Anonymität.
				</p>

				{#if isArmed(app.state)}
					<button type="button" class="panic" onclick={panic}>
						<Zap size={14} />
						Panik: Apps zu, Netz zu
					</button>
				{/if}
			</aside>
		</div>

		<footer class="statusbar">
			<span class="status-left">
				<span class="dot" style="background: {TONE_VAR[tone]}" aria-hidden="true"></span>
				{stateLabel} · seit {startLabel}
			</span>
			<span class="status-right">
				<span class="mode">Fokus-Modus · v{APP_VERSION}</span>
			</span>
		</footer>
	</div>
</div>

{#if app.toast}
	<div class="toast" role="status">{app.toast}</div>
{/if}

<SettingsSheet />
<DownloadSheet />
<SessionSummary />

<style>
	.stage {
		height: 100%;
		padding: 0;
		display: grid;
		place-items: center;
	}

	.window {
		position: relative;
		isolation: isolate;
		width: 100%;
		height: 100%;
		max-width: none;
		max-height: none;
		display: flex;
		flex-direction: column;
		border-radius: 14px;
		background-color: var(--window);
		background-image:
			radial-gradient(46% 40% at 14% 4%, color-mix(in oklab, var(--accent) 30%, transparent), transparent 70%),
			radial-gradient(38% 34% at 88% 6%, rgba(124, 92, 255, 0.26), transparent 70%),
			radial-gradient(52% 44% at 50% 104%, rgba(0, 199, 190, 0.18), transparent 72%);
		background-repeat: no-repeat;
		backdrop-filter: blur(30px) saturate(1.4);
		overflow: hidden;
	}

	.titlebar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px 10px;
	}

	.titlebar-left {
		display: flex;
		align-items: center;
		gap: 16px;
	}

	.window-controls {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.traffic {
		width: 12px;
		height: 12px;
		padding: 0;
		border: 0;
		border-radius: 50%;
		cursor: pointer;
		box-shadow: inset 0 0 0 0.5px rgba(0, 0, 0, 0.18);
		transition: filter 0.15s;
	}

	.traffic:hover {
		filter: brightness(0.92);
	}

	.traffic.close {
		background: #ff5f57;
	}

	.traffic.minimize {
		background: #febc2e;
	}

	.traffic.maximize {
		background: #28c840;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.logo {
		display: grid;
		place-items: center;
		width: 36px;
		height: 36px;
		border-radius: 11px;
		color: #fff;
		background: linear-gradient(145deg, #0a84ff, #7c5cff);
		box-shadow: 0 10px 24px -10px rgba(10, 132, 255, 0.9);
	}

	h1 {
		margin: 0;
		font-size: 15px;
		font-weight: 640;
		letter-spacing: -0.02em;
	}

	.sub {
		margin: 1px 0 0;
		font-size: 11.5px;
		color: var(--muted);
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.icon-button {
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border-radius: 50%;
		border: 1px solid var(--line);
		background: color-mix(in oklab, var(--card-strong) 70%, transparent);
		color: var(--muted);
		cursor: pointer;
		transition:
			color 0.2s,
			background 0.2s,
			transform 0.2s;
	}

	.icon-button:hover {
		color: var(--ink);
		background: var(--card-strong);
	}

	.icon-button:active {
		transform: scale(0.96);
	}

	.content {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: minmax(0, 1fr) 330px;
		gap: 18px;
		padding: 4px 20px 18px;
	}

	.alpha {
		display: flex;
		align-items: center;
		gap: 7px;
		margin: 0 20px 8px;
		padding: 7px 11px;
		border-radius: 10px;
		border: 1px solid color-mix(in oklab, var(--warn) 28%, transparent);
		background: color-mix(in oklab, var(--warn) 10%, transparent);
		color: color-mix(in oklab, var(--warn) 75%, var(--ink));
		font-size: 11.5px;
		font-weight: 520;
	}

	.card {
		border-radius: 22px;
		border: 1px solid var(--line);
		background: var(--card);
		box-shadow: var(--shadow-card);
	}

	.hero {
		min-height: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 16px;
		padding: 26px 28px;
		text-align: center;
	}

	.eyebrow {
		margin: 0;
		font-size: 10.5px;
		font-weight: 640;
		letter-spacing: 0.09em;
		text-transform: uppercase;
		color: var(--muted);
	}

	.headline h2 {
		margin: 0;
		font-size: 30px;
		font-weight: 690;
		letter-spacing: -0.03em;
		line-height: 1.1;
	}

	.hint {
		margin: 7px auto 0;
		max-width: 38ch;
		font-size: 13.5px;
		line-height: 1.5;
		color: var(--muted);
	}

	.banners {
		width: 100%;
		max-width: 420px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-top: 4px;
	}

	.error-code {
		margin: 0;
		font-size: 11.5px;
		color: var(--danger);
		font-variant-numeric: tabular-nums;
	}

	.side {
		display: flex;
		flex-direction: column;
		gap: 14px;
		min-height: 0;
		overflow-y: auto;
		padding-right: 2px;
	}

	.card-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 16px 4px;
	}

	.card-head h3 {
		margin: 0;
		font-size: 12.5px;
		font-weight: 640;
		letter-spacing: 0.01em;
	}

	.chip {
		padding: 3px 9px;
		border-radius: 999px;
		font-size: 10.5px;
		font-weight: 620;
		color: var(--accent);
		background: color-mix(in oklab, var(--accent) 12%, transparent);
	}

	.chip-button {
		padding: 3px 10px;
		border-radius: 999px;
		border: 1px solid var(--line);
		background: transparent;
		color: var(--muted);
		font: inherit;
		font-size: 10.5px;
		font-weight: 620;
		cursor: pointer;
		transition:
			color 0.2s,
			background 0.2s;
	}

	.chip-button:hover {
		color: var(--ink);
		background: color-mix(in oklab, var(--ink) 5%, transparent);
	}

	.card-body {
		padding: 2px 16px 12px;
	}

	.small-note {
		margin: 10px 0 0;
		font-size: 11.5px;
		line-height: 1.5;
		color: var(--muted);
	}

	.fineprint {
		margin: 0;
		padding: 0 6px;
		font-size: 11px;
		line-height: 1.5;
		color: var(--muted);
	}

	.panic {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 7px;
		padding: 10px 14px;
		border-radius: 14px;
		border: 1px solid color-mix(in oklab, var(--danger) 30%, transparent);
		background: color-mix(in oklab, var(--danger) 8%, transparent);
		color: var(--danger);
		font: inherit;
		font-size: 12.5px;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.2s;
	}

	.panic:hover {
		background: color-mix(in oklab, var(--danger) 15%, transparent);
	}

	.statusbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 22px;
		border-top: 1px solid var(--line);
		font-size: 11.5px;
		color: var(--muted);
	}

	.status-left {
		display: inline-flex;
		align-items: center;
		gap: 7px;
	}

	.status-right {
		display: inline-flex;
		align-items: center;
		gap: 12px;
	}

	.mode {
		font-variant-numeric: tabular-nums;
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
	}

	@keyframes pop-in {
		from {
			opacity: 0;
			transform: translateY(6px);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}

	.toast {
		position: fixed;
		left: 50%;
		bottom: 22px;
		transform: translateX(-50%);
		z-index: 40;
		max-width: 440px;
		padding: 9px 16px;
		border-radius: 999px;
		background: color-mix(in oklab, var(--ink) 92%, transparent);
		color: var(--canvas);
		font-size: 12px;
		font-weight: 520;
		box-shadow: 0 18px 40px -18px rgba(0, 0, 0, 0.6);
		animation: pop-in 0.2s var(--ease-out-soft);
	}

	@media (max-width: 880px) {
		.window {
			border-radius: 14px;
		}

		.content {
			display: flex;
			flex-direction: column;
			overflow-y: auto;
		}

		.hero,
		.side {
			flex: none;
		}

		.side {
			overflow: visible;
		}
	}
</style>
