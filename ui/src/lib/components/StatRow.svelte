<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		icon,
		label,
		value,
		tone = 'neutral',
		divider = true
	}: {
		icon: Snippet;
		label: string;
		value: string;
		tone?: 'neutral' | 'ok' | 'warn' | 'danger';
		divider?: boolean;
	} = $props();
</script>

<div class="row" class:divider>
	<span class="icon">{@render icon()}</span>
	<span class="label">{label}</span>
	<span class="value" data-tone={tone}>
		<span class="dot" aria-hidden="true"></span>
		{value}
	</span>
</div>

<style>
	.row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 9px 0;
	}

	.row.divider {
		border-top: 1px solid var(--line);
	}

	.icon {
		color: var(--muted);
		display: grid;
		place-items: center;
	}

	.label {
		font-size: 13px;
		color: var(--muted);
	}

	.value {
		margin-left: auto;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-size: 13px;
		font-weight: 560;
		font-variant-numeric: tabular-nums;
		color: var(--ink);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--muted);
	}

	.value[data-tone='ok'] .dot {
		background: var(--ok);
		box-shadow: 0 0 0 3px color-mix(in oklab, var(--ok) 22%, transparent);
	}

	.value[data-tone='warn'] .dot {
		background: var(--warn);
		box-shadow: 0 0 0 3px color-mix(in oklab, var(--warn) 22%, transparent);
	}

	.value[data-tone='danger'] .dot {
		background: var(--danger);
		box-shadow: 0 0 0 3px color-mix(in oklab, var(--danger) 22%, transparent);
	}
</style>
