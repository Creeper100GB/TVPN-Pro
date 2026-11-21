<script lang="ts">
	let {
		checked,
		onchange,
		label,
		disabled = false
	}: {
		checked: boolean;
		onchange: (value: boolean) => void;
		label: string;
		disabled?: boolean;
	} = $props();
</script>

<button
	type="button"
	class="toggle"
	class:on={checked}
	role="switch"
	aria-checked={checked}
	aria-label={label}
	{disabled}
	onclick={() => onchange(!checked)}
>
	<span class="knob"></span>
</button>

<style>
	.toggle {
		position: relative;
		width: 46px;
		height: 28px;
		flex: none;
		border: 0;
		border-radius: 999px;
		padding: 0;
		cursor: pointer;
		background: color-mix(in oklab, var(--ink) 16%, transparent);
		transition: background 0.25s var(--ease-out-soft);
	}

	.toggle.on {
		background: var(--ok);
	}

	.toggle:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.knob {
		position: absolute;
		top: 3px;
		left: 3px;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background: #fff;
		box-shadow:
			0 1px 3px rgba(15, 23, 42, 0.35),
			0 0 0 0.5px rgba(15, 23, 42, 0.08);
		transition: transform 0.28s var(--ease-out-soft);
	}

	.toggle.on .knob {
		transform: translateX(18px);
	}
</style>
