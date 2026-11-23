// Visual check for the UI shell. Serves the static build, drives the real
// enable/disable flow, captures light/dark screenshots and fails on console
// errors or layout overflow.
//
//   bun run preview          # in another terminal (port 4173)
//   bun run shots
//
// Screenshots land in ui/.artifacts/ (gitignored).

import { mkdir } from 'node:fs/promises';
import { chromium } from 'playwright';

const base = process.env.UI_BASE ?? 'http://localhost:4173';
const out = process.env.SHOT_OUT ?? '.artifacts';
const scale = Number(process.env.SHOT_SCALE ?? 2);

await mkdir(out, { recursive: true });

const browser = await chromium.launch({
	executablePath: process.env.CHROME_PATH || undefined
});
const page = await browser.newPage({
	viewport: { width: 1120, height: 780 },
	deviceScaleFactor: scale,
	colorScheme: 'light'
});

const problems = [];
page.on('console', (message) => {
	if (message.type() === 'error') problems.push(`console: ${message.text()}`);
});
page.on('pageerror', (error) => problems.push(`pageerror: ${error}`));

await page.goto(base, { waitUntil: 'networkidle' });

async function setTheme(mode) {
	await page.evaluate((value) => localStorage.setItem('tvpn-theme', value), mode);
	await page.reload({ waitUntil: 'networkidle' });
	await page.waitForTimeout(150);
}

async function waitForState(label) {
	await page.getByRole('heading', { name: label }).waitFor({ timeout: 20_000 });
	await page.waitForTimeout(250);
}

async function shot(name) {
	await page.screenshot({ path: `${out}/${name}.png` });
}

async function assertLayout(label) {
	const metrics = await page.evaluate(() => ({
		scrollWidth: document.documentElement.scrollWidth,
		innerWidth: window.innerWidth,
		scrollHeight: document.documentElement.scrollHeight,
		innerHeight: window.innerHeight,
		windowRadius: getComputedStyle(document.querySelector('.window')).borderRadius,
		bodyBg: getComputedStyle(document.body).backgroundColor,
		dark: document.documentElement.classList.contains('dark')
	}));

	if (metrics.scrollWidth > metrics.innerWidth + 1) {
		problems.push(`${label}: horizontal overflow (${metrics.scrollWidth} > ${metrics.innerWidth})`);
	}
	if (metrics.scrollHeight > metrics.innerHeight + 1) {
		problems.push(`${label}: vertical overflow (${metrics.scrollHeight} > ${metrics.innerHeight})`);
	}
	if (parseFloat(metrics.windowRadius) < 10) {
		problems.push(`${label}: window is not rounded (${metrics.windowRadius})`);
	}
	return metrics;
}

for (const theme of ['light', 'dark']) {
	await setTheme(theme);

	await waitForState('Aus');
	await shot(`${theme}-state-off`);

	await page.getByRole('button', { name: 'Schutz einschalten' }).click();
	await waitForState('Tor aktiv');
	await shot(`${theme}-state-ready`);
	await assertLayout(`${theme}/READY`);

	await page.getByRole('button', { name: 'Einstellungen öffnen' }).click();
	await page.waitForTimeout(350);
	await shot(`${theme}-settings`);
	await page.keyboard.press('Escape');
	await page.waitForTimeout(200);

	await page.getByRole('button', { name: 'Schutz ausschalten' }).click();
	await page.getByRole('dialog', { name: 'Sitzung beendet' }).waitFor({ timeout: 20_000 });
	await page.waitForTimeout(350);
	await shot(`${theme}-summary`);
	await page.keyboard.press('Escape');
	await waitForState('Aus');

	// Narrow window: layout must collapse to one column without sideways scrolling.
	await page.setViewportSize({ width: 520, height: 860 });
	await page.waitForTimeout(250);
	await shot(`${theme}-narrow`);
	await assertLayout(`${theme}/narrow`);
	await page.setViewportSize({ width: 1120, height: 780 });
	await page.waitForTimeout(200);
}

await assertLayout('final');

await browser.close();

if (problems.length > 0) {
	console.error('UI check failed:');
	for (const problem of problems) console.error(` - ${problem}`);
	process.exit(1);
}

console.log(`UI check passed. Screenshots in ${out}/`);
