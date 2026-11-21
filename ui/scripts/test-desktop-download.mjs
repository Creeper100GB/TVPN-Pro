// Drives the real Tauri window over the WebView2 debugging port and performs
// one HTTPS download end to end against a host that serves test files.
// Start the app with:
//   bun run tauri dev --config tauri.debug.conf.json
//
//   node scripts/test-desktop-download.mjs
//
// The downloaded file is removed again at the end.

import { chromium } from 'playwright';
import { existsSync } from 'node:fs';
import { readdir, rm, stat } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';

const downloadDir = path.join(os.homedir(), 'Downloads', 'TVPN');
const url = process.env.DOWNLOAD_URL ?? 'https://proof.ovh.net/files/1Mb.dat';
const expectedBytes = Number(process.env.EXPECTED_BYTES ?? 1024 * 1024);
const minimumBytes = Number(process.env.MINIMUM_BYTES ?? expectedBytes);

const browser = await chromium.connectOverCDP('http://127.0.0.1:9222', {
	timeout: 180_000
});

const page = browser
	.contexts()
	.flatMap((context) => context.pages())
	.find((candidate) => candidate.url().includes('localhost:5173'));

if (!page) throw new Error('TVPN window not found on the debugging port');

page.on('console', (message) => {
	if (message.type() === 'error') console.log('console:', message.text());
});

await page.waitForLoadState('domcontentloaded');
await page.locator('h2').first().waitFor({ timeout: 90_000 });

// Close a dialog left open by a previous run.
await page.keyboard.press('Escape');

const before = existsSync(downloadDir) ? new Set(await readdir(downloadDir)) : new Set();

if ((await page.locator('h2').first().innerText()).trim() === 'Aus') {
	await page.getByRole('button', { name: 'Schutz einschalten' }).click();
	await page.getByRole('heading', { name: 'Geschützt' }).waitFor({ timeout: 30_000 });
}

await page.getByRole('button', { name: 'Downloads öffnen' }).click();
const sheet = page.getByRole('dialog', { name: 'Downloads' });

const clear = sheet.getByRole('button', { name: 'Liste leeren' });
if (await clear.count()) await clear.click();

await sheet.getByRole('button', { name: 'Direkt' }).click();
await sheet.getByLabel('Download-URL').fill(url);
await sheet.getByRole('button', { name: 'Hinzufügen' }).click();

const item = sheet.getByTestId('download-item').first();
await item.waitFor({ timeout: 30_000 });
await item.getByText(/Sauber|Unklar|Fund/).waitFor({ timeout: 180_000 });

console.log('item:', (await item.innerText()).replace(/\n/g, ' | '));

const after = existsSync(downloadDir) ? await readdir(downloadDir) : [];
const created = after.filter((name) => !before.has(name));

if (created.length === 0) throw new Error('no file was written');

for (const name of created) {
	const file = path.join(downloadDir, name);
	const info = await stat(file);
	console.log(`file: ${name} (${info.size} bytes)`);
	if (info.size < minimumBytes) throw new Error(`expected at least ${minimumBytes} bytes`);
	if (expectedBytes > 0 && info.size !== expectedBytes && minimumBytes === expectedBytes) {
		throw new Error(`expected ${expectedBytes} bytes`);
	}
	await rm(file, { force: true });
	console.log('removed', file);
}

const left = existsSync(downloadDir) ? await readdir(downloadDir) : [];
console.log('remaining files in TVPN folder:', left.length);

process.exit(0);
