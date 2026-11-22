<script lang="ts">
	import { Check, ChevronDown, Copy } from 'lucide-svelte';
	import { COUNTRIES } from '../countries';
	import { isDesktop } from '../desktop';
	import { copyText, formatDiagnostics } from '../diagnostics';
	import { app, panic, updateSetting } from '../mock.svelte';
	import { setTheme, theme } from '../theme.svelte';
	import type { BlockFallback, MacMode, TorrentMode } from '../types';
	import { APP_VERSION } from '../version';
	import Segmented from './Segmented.svelte';
	import Sheet from './Sheet.svelte';
	import Toggle from './Toggle.svelte';

	const macOptions = [
		{ value: 'random' as MacMode, label: 'Neu würfeln' },
		{ value: 'original' as MacMode, label: 'Original' }
	];

	const torrentOptions = [
		{ value: 'blocked' as TorrentMode, label: 'Blockiert' },
		{ value: 'vpn_only' as TorrentMode, label: 'Nur VPN' }
	];

	const fallbackOptions = [
		{ value: 'off' as BlockFallback, label: 'Aus' },
		{ value: 'suggest' as BlockFallback, label: 'Vorschlag' },
		{ value: 'auto' as BlockFallback, label: 'Automatisch' }
	];

	const themeOptions = [
	{ value: 'auto' as const, label: 'Auto' },
	{ value: 'light' as const, label: 'Hell' },
	{ value: 'dark' as const, label: 'Dunkel' }
];

	let copyState = $state<'idle' | 'ok' | 'error'>('idle');
	let copyTimer: ReturnType<typeof setTimeout> | null = null;

	async function copyDiagnostics() {
		const report = formatDiagnostics({
			version: APP_VERSION,
			desktop: isDesktop,
			state: app.state,
			settings: app.settings,
			summary: app.lastSummary
		});

		try {
			await copyText(report);
			copyState = 'ok';
		} catch {
			copyState = 'error';
		}

		if (copyTimer) clearTimeout(copyTimer);
		copyTimer = setTimeout(() => (copyState = 'idle'), 4000);
	}
</script>

<Sheet
	bind:open={app.settingsOpen}
	title="Einstellungen"
	subtitle="Diese App blockt, statt zu hoffen. Was sie nicht leistet, steht im Threat Model."
>
	<section>
		<h3>Verbindung</h3>

		<div class="item">
			<div class="text">
				<p class="title">VPN</p>
				<p class="desc">VPN ist optional. Ohne VPN läuft Tor direkt; der ISP sieht Tor-Traffic.</p>
			</div>
			<Toggle
				checked={app.settings.vpnEnabled}
				label="VPN verwenden"
				onchange={(v) => updateSetting('vpnEnabled', v)}
			/>
		</div>

		<div class="item">
			<div class="text">
				<p class="title">Land</p>
				<p class="desc">Zufällig wählt pro Sitzung einen anderen Endpunkt.</p>
			</div>
			<div class="select">
				<select
					aria-label="VPN-Land"
					bind:value={app.settings.vpnCountry}
					disabled={!app.settings.vpnEnabled}
				>
					{#each COUNTRIES as country (country.code)}
						<option value={country.code}>{country.name}</option>
					{/each}
				</select>
				<ChevronDown size={15} aria-hidden="true" />
			</div>
		</div>

		<div class="item">
			<div class="text">
				<p class="title">Bridges</p>
				<p class="desc">Nur wenn Tor ohne Bridges blockiert wird.</p>
			</div>
			<Toggle
				checked={app.settings.torBridges}
				label="Tor-Bridges verwenden"
				onchange={(v) => updateSetting('torBridges', v)}
			/>
		</div>
	</section>

	<section>
		<h3>Schutz</h3>

		<div class="item">
			<div class="text">
				<p class="title">Firewall-Allowlist</p>
				<p class="desc">Blockt ausgehenden Verkehr außer Tor, VPN und Loopback. Loopback ist derzeit global erlaubt.</p>
			</div>
			<span class="badge">Immer an</span>
		</div>

		<div class="item">
			<div class="text">
				<p class="title">MAC-Adresse</p>
				<p class="desc">Der Hyper-V-Adapter unterstützt keinen Write; auf physischer Hardware ungeprüft.</p>
			</div>
			<Segmented
				label="MAC-Modus"
				options={macOptions}
				value={app.settings.macMode}
				onchange={(v) => updateSetting('macMode', v)}
			/>
		</div>

		<div class="item">
			<div class="text">
				<p class="title">Hostname</p>
				<p class="desc">Wird derzeit nicht geändert. Windows braucht dafür einen Neustart.</p>
			</div>
			<span class="badge">Unverändert</span>
		</div>

		<div class="item">
			<div class="text">
				<p class="title">Portal-Zeitfenster</p>
				<p class="desc">Nicht implementiert. Der Wert wird gespeichert, aber nicht verwendet.</p>
			</div>
			<div class="slider">
				<input
					type="range"
					min="60"
					max="600"
					step="30"
					aria-label="Portal-Zeitfenster in Sekunden"
					bind:value={app.settings.portalTimeoutSec}
				/>
				<span>{Math.round(app.settings.portalTimeoutSec / 60)} min</span>
			</div>
		</div>

		<div class="item">
			<div class="text">
				<p class="title">Netz-Modus</p>
				<p class="desc">Fokus: nur freigegebene Apps. System-Modus (TUN) kommt erst nach dem Audit.</p>
			</div>
			<span class="badge">Fokus</span>
		</div>
	</section>

	<section>
		<h3>Downloads</h3>

		<div class="item">
			<div class="text">
				<p class="title">Block-Fallback</p>
				<p class="desc">Nicht implementiert. Der Wert wird gespeichert, aber nicht verwendet.</p>
			</div>
			<Segmented
				label="Block-Fallback"
				options={fallbackOptions}
				value={app.settings.blockFallback}
				onchange={(v) => updateSetting('blockFallback', v)}
			/>
		</div>

		<div class="item">
			<div class="text">
				<p class="title">Torrents</p>
				<p class="desc">
					Nicht implementiert. Torrents werden derzeit nicht unterstützt.
				</p>
			</div>
			<Segmented
				label="Torrent-Modus"
				options={torrentOptions}
				value={app.settings.torrentMode}
				onchange={(v) => updateSetting('torrentMode', v)}
			/>
		</div>

		<div class="item">
			<div class="text">
				<p class="title">Hash-Abfrage</p>
				<p class="desc">Nicht implementiert. Es werden keine Hashes an Dritte gesendet.</p>
			</div>
			<Toggle
				checked={app.settings.scannerHashLookup}
				label="Hash-Abfrage erlauben"
				onchange={(v) => updateSetting('scannerHashLookup', v)}
			/>
		</div>
	</section>

	<section>
		<h3>Darstellung</h3>

		<div class="item">
			<div class="text">
				<p class="title">Erscheinungsbild</p>
				<p class="desc">Automatisch folgt Windows.</p>
			</div>
			<Segmented
				label="Erscheinungsbild"
				options={themeOptions}
				value={theme.mode}
				onchange={(v) => setTheme(v)}
				size="sm"
			/>
		</div>
	</section>

	<section>
		<h3>Notfall</h3>

		<div class="item">
			<div class="text">
				<p class="title">Panik-Taste</p>
				<p class="desc">Beendet Tor und VPN, entfernt die Firewall. Das Netz ist danach offen.</p>
			</div>
			<span class="badge">Strg + Alt + Shift + F12</span>
		</div>

		<button type="button" class="danger-button" onclick={panic}>Sofort beenden</button>
	</section>

	<section>
		<div class="tester-row">
			<button
				type="button"
				class="tester-button"
				title="Kopiert Version und Systemdaten für Fehlerberichte. Keine IP, MAC, URL oder Dateinamen."
				onclick={copyDiagnostics}
			>
				{#if copyState === 'ok'}
					<Check size={13} />
				{:else}
					<Copy size={13} />
				{/if}
				Tester-Infos kopieren
			</button>
			<span class="tester-feedback" role="status">
				{#if copyState === 'ok'}Kopiert.{:else if copyState === 'error'}Kopieren nicht möglich.{/if}
			</span>
		</div>
	</section>

	{#snippet footer()}
		<button type="button" class="done" onclick={() => (app.settingsOpen = false)}>Fertig</button>
	{/snippet}
</Sheet>

<style>
	section {
		padding: 14px 0;
	}

	section + section {
		border-top: 1px solid var(--line);
	}

	h3 {
		margin: 0 0 6px;
		font-size: 11px;
		font-weight: 640;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--muted);
	}

	.item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 18px;
		padding: 10px 0;
	}

	.text {
		min-width: 0;
	}

	.title {
		margin: 0;
		font-size: 13.5px;
		font-weight: 560;
	}

	.desc {
		margin: 2px 0 0;
		font-size: 12px;
		line-height: 1.4;
		color: var(--muted);
		max-width: 42ch;
	}

	.badge {
		flex: none;
		padding: 4px 10px;
		border-radius: 999px;
		border: 1px solid var(--line);
		background: color-mix(in oklab, var(--ink) 4%, transparent);
		font-size: 11.5px;
		font-weight: 560;
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}

	.select {
		position: relative;
		flex: none;
	}

	select {
		appearance: none;
		min-width: 150px;
		padding: 8px 30px 8px 12px;
		border-radius: 10px;
		border: 1px solid var(--line);
		background: var(--card);
		color: var(--ink);
		font: inherit;
		font-size: 12.5px;
		cursor: pointer;
	}

	select:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.select :global(svg) {
		position: absolute;
		right: 10px;
		top: 50%;
		transform: translateY(-50%);
		pointer-events: none;
		color: var(--muted);
	}

	.slider {
		display: flex;
		align-items: center;
		gap: 10px;
		flex: none;
	}

	.slider input {
		width: 130px;
		accent-color: var(--accent);
	}

	.slider span {
		font-size: 12px;
		color: var(--muted);
		font-variant-numeric: tabular-nums;
		min-width: 44px;
		text-align: right;
	}

	.danger-button {
		margin-top: 6px;
		width: 100%;
		padding: 10px;
		border-radius: 12px;
		border: 1px solid color-mix(in oklab, var(--danger) 35%, transparent);
		background: color-mix(in oklab, var(--danger) 9%, transparent);
		color: var(--danger);
		font: inherit;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.2s;
	}

	.danger-button:hover {
		background: color-mix(in oklab, var(--danger) 15%, transparent);
	}

	.done {
		padding: 9px 20px;
		border-radius: 999px;
		border: 0;
		background: var(--accent);
		color: #fff;
		font: inherit;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition:
			filter 0.2s,
			transform 0.2s;
	}

	.done:hover {
		filter: brightness(1.06);
	}

	.done:active {
		transform: scale(0.98);
	}

	.tester-row {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.tester-feedback {
		font-size: 11.5px;
		color: var(--muted);
	}

	.tester-button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 6px 11px;
		border-radius: 999px;
		border: 1px solid var(--line);
		background: transparent;
		color: var(--muted);
		font: inherit;
		font-size: 11.5px;
		font-weight: 560;
		cursor: pointer;
		transition:
			color 0.2s,
			background 0.2s;
	}

	.tester-button:hover {
		color: var(--ink);
		background: color-mix(in oklab, var(--ink) 5%, transparent);
	}
</style>
