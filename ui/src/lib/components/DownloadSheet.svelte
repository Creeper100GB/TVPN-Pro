<script lang="ts">
	import { Download, Trash2, X } from 'lucide-svelte';
	import { addDownload, app, clearDownloads, removeDownload } from '../mock.svelte';
	import type { DownloadPhase, ScanVerdict } from '../types';
	import Sheet from './Sheet.svelte';

	let url = $state('');
	let error = $state<string | null>(null);

	const verdictLabel: Record<ScanVerdict, string> = {
		clean: 'Sauber',
		warned: 'Warnung',
		quarantined: 'Quarantäne',
		failed: 'Fehlgeschlagen'
	};

	const phaseLabel: Record<DownloadPhase, string> = {
		queued: 'Wartet',
		active: 'Lädt',
		paused: 'Pausiert',
		done: 'Fertig',
		error: 'Fehler'
	};

	function submit() {
		const result = addDownload(url.trim());
		if (!result.ok) {
			error = result.error;
			return;
		}
		url = '';
		error = null;
	}
</script>

<Sheet
	bind:open={app.downloadsOpen}
	title="Downloads"
	subtitle="Links laufen über Tor. Erst der Scan entscheidet, wohin die Datei kommt."
>
	<form
		class="add"
		onsubmit={(event) => {
			event.preventDefault();
			submit();
		}}
	>
		<input
			bind:value={url}
			type="url"
			placeholder="https://…"
			aria-label="Download-URL"
			autocomplete="off"
			spellcheck="false"
		/>
		<button type="submit">Hinzufügen</button>
	</form>

	{#if error}
		<p class="error" role="alert">{error}</p>
	{/if}

	{#if app.downloads.length === 0}
		<div class="empty">
			<Download size={22} strokeWidth={1.6} />
			<p>Keine Downloads in dieser Sitzung.</p>
		</div>
	{:else}
		<ul class="list">
			{#each app.downloads as item (item.id)}
				<li data-testid="download-item">
					<div class="row">
						<span class="name" title={item.label}>{item.label}</span>
						{#if item.verdict}
							<span class="verdict" data-verdict={item.verdict}>{verdictLabel[item.verdict]}</span>
						{:else}
							<span class="verdict neutral">{phaseLabel[item.phase]}</span>
						{/if}
						<button
							type="button"
							class="remove"
							aria-label="{item.label} entfernen"
							onclick={() => removeDownload(item.id)}
						>
							<X size={14} />
						</button>
					</div>

					{#if item.phase !== 'done'}
						<div
							class="bar"
							role="progressbar"
							aria-valuenow={item.progress}
							aria-valuemin="0"
							aria-valuemax="100"
							aria-label="Fortschritt {item.label}"
						>
							<span style="width: {item.progress}%"></span>
						</div>
					{/if}
				</li>
			{/each}
		</ul>

		<button type="button" class="clear" onclick={clearDownloads}>
			<Trash2 size={14} />
			Liste leeren
		</button>
	{/if}

	{#snippet footer()}
		<button type="button" class="done" onclick={() => (app.downloadsOpen = false)}>Fertig</button>
	{/snippet}
</Sheet>

<style>
	.add {
		display: flex;
		gap: 8px;
		margin: 6px 0 4px;
	}

	input {
		flex: 1;
		min-width: 0;
		padding: 9px 12px;
		border-radius: 10px;
		border: 1px solid var(--line);
		background: var(--card);
		color: var(--ink);
		font: inherit;
		font-size: 13px;
	}

	input::placeholder {
		color: var(--muted);
	}

	.add button {
		flex: none;
		padding: 9px 16px;
		border: 0;
		border-radius: 10px;
		background: var(--accent);
		color: #fff;
		font: inherit;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
	}

	.error {
		margin: 6px 0 0;
		font-size: 12px;
		color: var(--danger);
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		padding: 34px 0 26px;
		color: var(--muted);
	}

	.empty p {
		margin: 0;
		font-size: 13px;
	}

	.list {
		list-style: none;
		margin: 10px 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-height: 300px;
		overflow-y: auto;
	}

	li {
		padding: 10px 12px;
		border-radius: 12px;
		border: 1px solid var(--line);
		background: color-mix(in oklab, var(--ink) 3%, transparent);
	}

	.row {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 13px;
		font-weight: 540;
	}

	.verdict {
		margin-left: auto;
		flex: none;
		padding: 3px 9px;
		border-radius: 999px;
		font-size: 11px;
		font-weight: 620;
		color: var(--ok);
		background: color-mix(in oklab, var(--ok) 12%, transparent);
	}

	.verdict.neutral {
		color: var(--muted);
		background: color-mix(in oklab, var(--ink) 6%, transparent);
	}

	.verdict[data-verdict='warned'] {
		color: var(--warn);
		background: color-mix(in oklab, var(--warn) 14%, transparent);
	}

	.verdict[data-verdict='quarantined'],
	.verdict[data-verdict='failed'] {
		color: var(--danger);
		background: color-mix(in oklab, var(--danger) 12%, transparent);
	}

	.remove {
		flex: none;
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border: 0;
		border-radius: 50%;
		background: transparent;
		color: var(--muted);
		cursor: pointer;
	}

	.remove:hover {
		color: var(--danger);
		background: color-mix(in oklab, var(--danger) 10%, transparent);
	}

	.bar {
		margin-top: 8px;
		height: 4px;
		border-radius: 999px;
		background: color-mix(in oklab, var(--ink) 8%, transparent);
		overflow: hidden;
	}

	.bar span {
		display: block;
		height: 100%;
		border-radius: inherit;
		background: var(--accent);
		transition: width 0.3s var(--ease-out-soft);
	}

	.clear {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		margin-top: 12px;
		padding: 7px 12px;
		border-radius: 999px;
		border: 1px solid color-mix(in oklab, var(--danger) 28%, transparent);
		background: transparent;
		color: var(--danger);
		font: inherit;
		font-size: 12px;
		font-weight: 560;
		cursor: pointer;
	}

	.clear:hover {
		background: color-mix(in oklab, var(--danger) 8%, transparent);
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
