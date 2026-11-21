<script lang="ts">
	import { Power } from 'lucide-svelte';
	import { TONE_VAR, isTransient } from '../states';
	import type { AppState, Tone } from '../types';

	let {
		state,
		progress,
		tone,
		onclick,
		disabled = false,
		label
	}: {
		state: AppState;
		progress: number;
		tone: Tone;
		onclick: () => void;
		disabled?: boolean;
		label: string;
	} = $props();

	const busy = $derived(isTransient(state));
</script>

<button
	type="button"
	class="power"
	class:busy
	style="--tone: {TONE_VAR[tone]}; --p: {Math.max(0, Math.min(100, progress))}"
	aria-label={label}
	{disabled}
	onclick={onclick}
>
	<span class="ring" aria-hidden="true"></span>
	<span class="disc">
		<Power size={54} strokeWidth={1.5} />
	</span>
</button>

<style>
	.power {
		position: relative;
		display: grid;
		place-items: center;
		width: 208px;
		height: 208px;
		padding: 0;
		border: 0;
		border-radius: 50%;
		background: none;
		cursor: pointer;
		isolation: isolate;
	}

	.power:disabled {
		cursor: progress;
	}

	.ring {
		position: absolute;
		inset: 0;
		border-radius: 50%;
		background: conic-gradient(
			from 180deg,
			var(--tone) calc(var(--p) * 1%),
			color-mix(in oklab, var(--tone) 14%, transparent) 0
		);
		-webkit-mask: radial-gradient(farthest-side, transparent calc(100% - 16px), #000 calc(100% - 15px));
		mask: radial-gradient(farthest-side, transparent calc(100% - 16px), #000 calc(100% - 15px));
		opacity: 0.9;
		transition: background 0.5s var(--ease-out-soft);
	}

	.disc {
		position: relative;
		display: grid;
		place-items: center;
		width: 168px;
		height: 168px;
		border-radius: 50%;
		color: #fff;
		background: radial-gradient(
			120% 120% at 30% 18%,
			color-mix(in oklab, var(--tone) 78%, white),
			var(--tone) 52%,
			color-mix(in oklab, var(--tone) 74%, black)
		);
		box-shadow:
			0 26px 60px -22px color-mix(in oklab, var(--tone) 65%, transparent),
			inset 0 1px 0 rgba(255, 255, 255, 0.45),
			inset 0 -3px 10px rgba(0, 0, 0, 0.18);
		transition:
			transform 0.3s var(--ease-out-soft),
			background 0.5s var(--ease-out-soft),
			box-shadow 0.4s var(--ease-out-soft);
	}

	.power:hover .disc {
		transform: translateY(-2px) scale(1.012);
		box-shadow:
			0 32px 70px -22px color-mix(in oklab, var(--tone) 72%, transparent),
			inset 0 1px 0 rgba(255, 255, 255, 0.5),
			inset 0 -3px 10px rgba(0, 0, 0, 0.18);
	}

	.power:active .disc {
		transform: scale(0.984);
	}

	.busy .disc {
		animation: breathe 2.6s ease-in-out infinite;
	}

	@keyframes breathe {
		0%,
		100% {
			box-shadow:
				0 26px 60px -22px color-mix(in oklab, var(--tone) 55%, transparent),
				inset 0 1px 0 rgba(255, 255, 255, 0.45),
				inset 0 -3px 10px rgba(0, 0, 0, 0.18);
		}
		50% {
			box-shadow:
				0 30px 80px -18px color-mix(in oklab, var(--tone) 85%, transparent),
				inset 0 1px 0 rgba(255, 255, 255, 0.45),
				inset 0 -3px 10px rgba(0, 0, 0, 0.18);
			transform: translateY(-1px);
		}
	}
</style>
