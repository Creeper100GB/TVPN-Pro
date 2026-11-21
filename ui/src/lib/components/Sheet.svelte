<script lang="ts">
	import { X } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	let {
		open = $bindable(false),
		title,
		subtitle,
		children,
		footer
	}: {
		open?: boolean;
		title: string;
		subtitle?: string;
		children: Snippet;
		footer?: Snippet;
	} = $props();

	let dialog = $state<HTMLDialogElement | null>(null);

	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) dialog.showModal();
		if (!open && dialog.open) dialog.close();
	});

	function close() {
		open = false;
	}
</script>

<dialog bind:this={dialog} class="sheet" onclose={() => (open = false)} aria-label={title}>
	<div class="sheet-body">
		<header class="sheet-head">
			<div>
				<h2>{title}</h2>
				{#if subtitle}<p>{subtitle}</p>{/if}
			</div>
			<button type="button" class="close" aria-label="Schließen" onclick={close}>
				<X size={18} />
			</button>
		</header>

		<div class="sheet-content">
			{@render children()}
		</div>

		{#if footer}
			<footer class="sheet-foot">
				{@render footer()}
			</footer>
		{/if}
	</div>
</dialog>

<style>
	.sheet {
		width: min(560px, calc(100vw - 32px));
		max-height: calc(100vh - 48px);
		margin: auto;
		padding: 0;
		border: 1px solid var(--line);
		border-radius: 24px;
		background: var(--card-strong);
		color: var(--ink);
		box-shadow: 0 40px 120px -30px rgba(0, 0, 0, 0.55);
		overflow: hidden;
		animation: sheet-in 0.28s var(--ease-out-soft);
	}

	.sheet::backdrop {
		background: rgba(8, 10, 14, 0.35);
		backdrop-filter: blur(8px);
	}

	.sheet-body {
		display: flex;
		max-height: inherit;
		flex-direction: column;
	}

	.sheet-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 16px;
		padding: 22px 24px 12px;
	}

	h2 {
		margin: 0;
		font-size: 19px;
		font-weight: 640;
		letter-spacing: -0.022em;
	}

	p {
		margin: 4px 0 0;
		font-size: 13px;
		color: var(--muted);
		max-width: 42ch;
	}

	.close {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		flex: none;
		border: 0;
		border-radius: 50%;
		background: color-mix(in oklab, var(--ink) 7%, transparent);
		color: var(--muted);
		cursor: pointer;
		transition:
			background 0.2s,
			color 0.2s;
	}

	.close:hover {
		background: color-mix(in oklab, var(--ink) 12%, transparent);
		color: var(--ink);
	}

	.sheet-content {
		overflow-y: auto;
		padding: 4px 24px 18px;
	}

	.sheet-foot {
		display: flex;
		justify-content: flex-end;
		gap: 10px;
		padding: 14px 24px 20px;
		border-top: 1px solid var(--line);
		background: color-mix(in oklab, var(--ink) 3%, transparent);
	}

	@keyframes sheet-in {
		from {
			opacity: 0;
			transform: translateY(10px) scale(0.985);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}
</style>
