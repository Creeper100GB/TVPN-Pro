<script lang="ts">
	import { CheckCircle2, Download, ShieldAlert, TriangleAlert } from 'lucide-svelte';
	import { app } from '../mock.svelte';
	import Sheet from './Sheet.svelte';

	const summary = $derived(app.lastSummary);

	function duration(from: number, to: number) {
		const seconds = Math.max(0, Math.round((to - from) / 1000));
		const minutes = Math.floor(seconds / 60);
		return minutes > 0 ? `${minutes} min ${seconds % 60} s` : `${seconds} s`;
	}
</script>

<Sheet
	bind:open={app.summaryOpen}
	title="Sitzung beendet"
	subtitle="Was in dieser Sitzung passiert ist. Ohne Ziele, ohne Dateinamen."
>
	{#if summary}
		<div class="grid">
			<div class="tile">
				<p class="k">Dauer</p>
				<p class="v">{duration(summary.startedAt, summary.endedAt)}</p>
			</div>
			<div class="tile">
				<p class="k">Erreichter Zustand</p>
				<p class="v">{summary.stateReached}</p>
			</div>
		</div>

		<ul class="rows">
			<li>
				<span><CheckCircle2 size={16} /> Sauber</span>
				<b>{summary.clean}</b>
			</li>
			<li>
				<span><TriangleAlert size={16} /> Mit Warnung</span>
				<b>{summary.warned}</b>
			</li>
			<li>
				<span><ShieldAlert size={16} /> Quarantäne</span>
				<b>{summary.quarantined}</b>
			</li>
			<li>
				<span><Download size={16} /> Fehlgeschlagen</span>
				<b>{summary.failed}</b>
			</li>
		</ul>

		<p class="note">
			Das Netz ist wieder normal. Ab jetzt schützt die Firewall nicht mehr: Dokumente, die beim
			Öffnen nachladen, sehen deine echte IP.
		</p>
	{:else}
		<p class="note">Keine Sitzungsdaten vorhanden.</p>
	{/if}

	{#snippet footer()}
		<button type="button" class="done" onclick={() => (app.summaryOpen = false)}>Fertig</button>
	{/snippet}
</Sheet>

<style>
	.grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
		margin: 6px 0 14px;
	}

	.tile {
		padding: 12px 14px;
		border-radius: 14px;
		border: 1px solid var(--line);
		background: color-mix(in oklab, var(--ink) 3%, transparent);
	}

	.k {
		margin: 0;
		font-size: 11px;
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 600;
	}

	.v {
		margin: 4px 0 0;
		font-size: 16px;
		font-weight: 640;
		font-variant-numeric: tabular-nums;
	}

	.rows {
		list-style: none;
		margin: 0;
		padding: 0;
		border: 1px solid var(--line);
		border-radius: 14px;
		overflow: hidden;
	}

	.rows li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 14px;
		font-size: 13px;
	}

	.rows li + li {
		border-top: 1px solid var(--line);
	}

	.rows span {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		color: var(--muted);
	}

	.rows b {
		font-variant-numeric: tabular-nums;
	}

	.note {
		margin: 14px 0 0;
		font-size: 12px;
		line-height: 1.5;
		color: var(--muted);
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
	}
</style>
