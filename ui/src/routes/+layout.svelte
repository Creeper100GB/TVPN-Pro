<script lang="ts">
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import '../app.css';
	import { theme } from '../lib/theme.svelte';

	let { children }: { children: Snippet } = $props();

	function apply() {
		if (typeof document === 'undefined') return;
		const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
		const dark = theme.mode === 'dark' || (theme.mode === 'auto' && prefersDark);
		document.documentElement.classList.toggle('dark', dark);
	}

	$effect(() => {
		// re-run when the mode changes
		void theme.mode;
		apply();
	});

	onMount(() => {
		const media = window.matchMedia('(prefers-color-scheme: dark)');
		const onChange = () => apply();
		media.addEventListener('change', onChange);
		apply();
		return () => media.removeEventListener('change', onChange);
	});
</script>

{@render children()}
