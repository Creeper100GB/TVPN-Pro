<script lang="ts">
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import '../app.css';
	import { isDesktop } from '../lib/desktop';
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
		// Tauri renders the window frameless and transparent; the panel paints its own light.
		if ('__TAURI_INTERNALS__' in window) {
			document.documentElement.classList.add('desktop');
		}
		const media = window.matchMedia('(prefers-color-scheme: dark)');
		const onChange = () => apply();
		media.addEventListener('change', onChange);
		apply();
		return () => media.removeEventListener('change', onChange);
	});

	onMount(() => {
		if (!isDesktop) return;

		let disposed = false;
		let stop: (() => void) | undefined;

		void (async () => {
			try {
				const { getCurrentWindow } = await import('@tauri-apps/api/window');
				const windowHandle = getCurrentWindow();

				const sync = async () => {
					try {
						const full =
							(await windowHandle.isFullscreen()) || (await windowHandle.isMaximized());
						document.documentElement.classList.toggle('window-full', full);
					} catch {
						// Missing permission: keep the rounded look instead of breaking layout.
					}
				};

				await sync();
				const unlisten = await windowHandle.onResized(() => {
					void sync();
				});
				if (disposed) {
					unlisten();
					return;
				}
				stop = unlisten;
			} catch {
				// Keep the rounded corners if the window API is unavailable.
			}
		})();

		return () => {
			disposed = true;
			stop?.();
		};
	});
</script>

{@render children()}
