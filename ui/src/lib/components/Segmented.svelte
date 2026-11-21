<script lang="ts" generics="T extends string">
	let {
		options,
		value,
		onchange,
		label,
		size = 'md'
	}: {
		options: Array<{ value: T; label: string }>;
		value: T;
		onchange: (value: T) => void;
		label: string;
		size?: 'sm' | 'md';
	} = $props();
</script>

<div class="seg" class:sm={size === 'sm'} role="group" aria-label={label}>
	{#each options as option (option.value)}
		<button
			type="button"
			class="seg-item"
			class:active={option.value === value}
			aria-pressed={option.value === value}
			onclick={() => onchange(option.value)}
		>
			{option.label}
		</button>
	{/each}
</div>

<style>
	.seg {
		display: inline-flex;
		gap: 2px;
		padding: 3px;
		border-radius: 999px;
		background: color-mix(in oklab, var(--ink) 6%, transparent);
		border: 1px solid var(--line);
	}

	.seg-item {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		white-space: nowrap;
		border: 0;
		border-radius: 999px;
		padding: 7px 14px;
		font-size: 13px;
		font-weight: 520;
		color: var(--muted);
		background: transparent;
		cursor: pointer;
		transition:
			background 0.22s var(--ease-out-soft),
			color 0.22s var(--ease-out-soft),
			box-shadow 0.22s var(--ease-out-soft);
	}

	.sm .seg-item {
		padding: 5px 10px;
		font-size: 12px;
	}

	.seg-item:hover {
		color: var(--ink);
	}

	.seg-item.active {
		color: var(--ink);
		background: var(--card-strong);
		box-shadow:
			0 1px 2px rgba(15, 23, 42, 0.12),
			0 4px 12px -6px rgba(15, 23, 42, 0.25);
	}

	:global(.dark) .seg-item.active {
		box-shadow:
			0 1px 2px rgba(0, 0, 0, 0.5),
			inset 0 1px 0 rgba(255, 255, 255, 0.08);
	}
</style>
