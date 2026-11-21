<script lang="ts">
	import {
		Activity,
		ChevronDown,
		Download,
		FlaskConical,
		Globe,
		Network,
		Settings as SettingsIcon,
		ShieldCheck,
		Wifi
	} from 'lucide-svelte';
	import { countryName } from '../lib/countries';
	import { app, disable, enable, jumpTo, panic, recover, showToast } from '../lib/mock.svelte';
	import { STATE_META, TONE_VAR, isArmed, isTransient } from '../lib/states';
	import { STATE_ORDER } from '../lib/types';
	import Banner from '../lib/components/Banner.svelte';
	import PowerButton from '../lib/components/PowerButton.svelte';
	import SessionSummary from '../lib/components/SessionSummary.svelte';
	import SettingsSheet from '../lib/components/SettingsSheet.svelte';
	import StatRow from '../lib/components/StatRow.svelte';

	let demoOpen = $state(false);

	const meta = $derived(STATE_META[app.state]);
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

	const primaryBackground = $derived(
		app.state === 'OFF'
			? 'var(--accent)'
			: app.state === 'FAILCLOSED'
				? 'var(--danger)'
				: 'color-mix(in oklab, var(--ink) 88%, transparent)'
	);

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

<svelte:window onkeydown={(event) => event.key === 'Escape' && (demoOpen = false)} />

<div class="stage">
	<div class="window">
		<header class="titlebar">
			<div class="brand">
				<div class="logo"><ShieldCheck size={19} strokeWidth={2.1} /></div>
				<div>
					<h1>TVPN Pro</h1>
					<p class="sub">Fokus-Modus · keine Telemetrie</p>
				</div>
			</div>

			<div class="actions">
				<span class="demo-chip" title="Es ist noch kein Dienst verbunden.">Demo</span>
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

		<div class="content">
			<section class="hero card" aria-labelledby="state-title">
				<p class="eyebrow">Status</p>

				<PowerButton
					state={app.state}
					progress={app.progress}
					{tone}
					label={meta.action}
					disabled={primaryDisabled}
					onclick={primary}
				/>

				<div class="headline">
					<h2 id="state-title" aria-live="polite">{meta.label}</h2>
					<p class="hint">{meta.hint}</p>
				</div>

				<div class="buttons">
					<button
						type="button"
						class="primary"
						disabled={primaryDisabled}
						style="background: {primaryBackground}"
						onclick={primary}
					>
						{meta.action}
					</button>

					{#if app.state === 'READY' || app.state === 'DEGRADED'}
						<button type="button" class="ghost" onclick={panic}>Panik</button>
					{/if}
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
						<span class="chip neutral">{app.downloads.active} aktiv</span>
					</header>
					<div class="card-body">
						<StatRow
							label="In Quarantäne"
							value={String(app.downloads.quarantined)}
							tone={app.downloads.quarantined > 0 ? 'warn' : 'neutral'}
							divider={false}
						>
							{#snippet icon()}<Download size={15} strokeWidth={1.8} />{/snippet}
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
			</aside>
		</div>

		<footer class="statusbar">
			<span class="status-left">
				<span class="dot" style="background: {TONE_VAR[tone]}" aria-hidden="true"></span>
				{meta.label} · seit {startLabel}
			</span>
			<span class="status-right">Fokus-Modus · v0.0.0</span>
		</footer>
	</div>
</div>

<div class="demo">
	{#if demoOpen}
		<div class="popover" role="dialog" aria-label="Demo-Zustände">
			<p class="pop-title">Zustand ansehen</p>
			<div class="pop-grid">
				{#each STATE_ORDER as state (state)}
					<button
						type="button"
						class="pop-state"
						class:current={state === app.state}
						onclick={() => jumpTo(state)}
					>
						{state}
					</button>
				{/each}
			</div>
			<label class="pop-toggle">
				<input type="checkbox" bind:checked={app.demoPortal} />
				Portal-Schritt simulieren
			</label>
			<button
				type="button"
				class="pop-state wide"
				onclick={() => {
					app.lastSummary = {
						startedAt: Date.now() - 754000,
						endedAt: Date.now(),
						stateReached: 'READY',
						clean: 3,
						warned: 1,
						quarantined: 1,
						failed: 0,
						failclosedCount: 1,
						reasons: ['E_TOR_BOOTSTRAP_TIMEOUT']
					};
					app.summaryOpen = true;
					demoOpen = false;
				}}
			>
				Sitzungs-Zusammenfassung zeigen
			</button>
		</div>
	{/if}

	<button
		type="button"
		class="fab"
		aria-expanded={demoOpen}
		aria-label="Demo-Zustände"
		onclick={() => (demoOpen = !demoOpen)}
	>
		<FlaskConical size={15} />
		Demo
		<ChevronDown size={14} class="chev {demoOpen ? 'up' : ''}" />
	</button>
</div>

{#if app.toast}
	<div class="toast" role="status">{app.toast}</div>
{/if}

<SettingsSheet />
<SessionSummary />

<style>
	.stage {
		height: 100%;
		padding: 22px;
		display: grid;
		place-items: center;
	}

	.window {
		width: 100%;
		max-width: 1060px;
		height: 100%;
		max-height: 740px;
		display: flex;
		flex-direction: column;
		border-radius: 30px;
		border: 1px solid var(--line);
		background: var(--window);
		backdrop-filter: blur(30px) saturate(1.4);
		box-shadow: var(--shadow-window);
		overflow: hidden;
	}

	.titlebar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px 10px;
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

	.demo-chip {
		padding: 4px 9px;
		border-radius: 999px;
		font-size: 10.5px;
		font-weight: 620;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--muted);
		background: color-mix(in oklab, var(--ink) 6%, transparent);
		border: 1px solid var(--line);
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

	.buttons {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.primary {
		height: 44px;
		padding: 0 26px;
		border: 0;
		border-radius: 999px;
		color: #fff;
		font: inherit;
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		box-shadow: 0 14px 30px -14px rgba(10, 132, 255, 0.75);
		transition:
			transform 0.2s var(--ease-out-soft),
			filter 0.2s;
	}

	.primary:hover:not(:disabled) {
		filter: brightness(1.05);
		transform: translateY(-1px);
	}

	.primary:active:not(:disabled) {
		transform: scale(0.985);
	}

	.primary:disabled {
		opacity: 0.55;
		cursor: progress;
		box-shadow: none;
	}

	.ghost {
		height: 44px;
		padding: 0 18px;
		border-radius: 999px;
		border: 1px solid var(--line);
		background: transparent;
		color: var(--muted);
		font: inherit;
		font-size: 13px;
		font-weight: 560;
		cursor: pointer;
		transition:
			color 0.2s,
			background 0.2s;
	}

	.ghost:hover {
		color: var(--ink);
		background: color-mix(in oklab, var(--ink) 5%, transparent);
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

	.chip.neutral {
		color: var(--muted);
		background: color-mix(in oklab, var(--ink) 6%, transparent);
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

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
	}

	.demo {
		position: fixed;
		right: 18px;
		bottom: 18px;
		z-index: 30;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 8px;
	}

	.fab {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 8px 12px;
		border-radius: 999px;
		border: 1px solid var(--line);
		background: var(--card-strong);
		color: var(--muted);
		font: inherit;
		font-size: 12px;
		font-weight: 560;
		cursor: pointer;
		box-shadow: var(--shadow-card);
	}

	.fab:hover {
		color: var(--ink);
	}

	.fab :global(.chev) {
		transition: transform 0.2s;
	}

	.fab :global(.chev.up) {
		transform: rotate(180deg);
	}

	.popover {
		width: 300px;
		padding: 12px;
		border-radius: 18px;
		border: 1px solid var(--line);
		background: var(--card-strong);
		box-shadow: 0 24px 60px -24px rgba(0, 0, 0, 0.5);
		animation: pop-in 0.2s var(--ease-out-soft);
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

	.pop-title {
		margin: 0 0 8px;
		font-size: 11px;
		font-weight: 640;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--muted);
	}

	.pop-grid {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 5px;
	}

	.pop-state {
		padding: 6px 4px;
		border-radius: 8px;
		border: 1px solid var(--line);
		background: transparent;
		color: var(--muted);
		font: inherit;
		font-size: 10px;
		font-weight: 560;
		cursor: pointer;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.pop-state:hover {
		color: var(--ink);
		background: color-mix(in oklab, var(--ink) 5%, transparent);
	}

	.pop-state.current {
		color: var(--accent);
		border-color: color-mix(in oklab, var(--accent) 40%, transparent);
		background: color-mix(in oklab, var(--accent) 8%, transparent);
	}

	.pop-state.wide {
		width: 100%;
		margin-top: 8px;
		padding: 8px;
		font-size: 11px;
	}

	.pop-toggle {
		display: flex;
		align-items: center;
		gap: 7px;
		margin-top: 9px;
		font-size: 11.5px;
		color: var(--muted);
	}

	.pop-toggle input {
		accent-color: var(--accent);
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
		.stage {
			padding: 10px;
		}

		.window {
			max-height: none;
			border-radius: 24px;
		}

		.content {
			grid-template-columns: minmax(0, 1fr);
			overflow-y: auto;
		}
	}
</style>
