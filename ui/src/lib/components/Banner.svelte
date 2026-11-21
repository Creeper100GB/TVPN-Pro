<script lang="ts">
	import { Activity, Fingerprint, Globe, Monitor, ShieldAlert, Wifi } from 'lucide-svelte';
	import type { BannerKind } from '../types';

	let { kind }: { kind: BannerKind } = $props();

	interface BannerMeta {
		icon: typeof Wifi;
		tone: 'warn' | 'danger';
		text: string;
	}

	const map: Record<BannerKind, BannerMeta> = {
		portal_open: {
			icon: Wifi,
			tone: 'warn',
			text: 'Portal-Anmeldung offen. Das Portal sieht dein Gerät ohne VPN.'
		},
		fallback_active: {
			icon: Globe,
			tone: 'warn',
			text: 'VPN-Browser aktiv. Diese Seite sieht deine VPN-IP – nicht mit denselben Konten anmelden.'
		},
		degraded: {
			icon: Activity,
			tone: 'warn',
			text: 'Eine Komponente ist ausgefallen und wird neu gestartet. Das Netz bleibt gesperrt.'
		},
		failclosed: {
			icon: ShieldAlert,
			tone: 'danger',
			text: 'Netz gesperrt. Nichts geht raus, bis du entscheidest.'
		},
		mac_unsupported: {
			icon: Fingerprint,
			tone: 'warn',
			text: 'Der Adaptertreiber kann die MAC nicht ändern. Betrieb nur mit Bestätigung.'
		},
		hostname_unchanged: {
			icon: Monitor,
			tone: 'warn',
			text: 'Hostname unverändert. Sitzungen im selben Netz können verknüpft werden.'
		}
	};

	const meta = $derived(map[kind]);
	const Icon = $derived(meta.icon);
</script>

<div class="banner" data-tone={meta.tone} role="status">
	<Icon size={17} />
	<p>{meta.text}</p>
</div>

<style>
	.banner {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		padding: 11px 14px;
		border-radius: 14px;
		border: 1px solid color-mix(in oklab, var(--tone) 26%, transparent);
		background: color-mix(in oklab, var(--tone) 11%, transparent);
		color: color-mix(in oklab, var(--tone) 72%, var(--ink));
	}

	.banner[data-tone='warn'] {
		--tone: var(--warn);
	}

	.banner[data-tone='danger'] {
		--tone: var(--danger);
	}

	.banner :global(svg) {
		flex: none;
		margin-top: 1px;
	}

	p {
		margin: 0;
		font-size: 12.5px;
		line-height: 1.45;
		font-weight: 500;
	}
</style>
