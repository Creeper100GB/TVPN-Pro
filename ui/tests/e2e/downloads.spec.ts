import { expect, test } from '@playwright/test';
import { createServer, type Server } from 'node:http';
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// A deterministic 256 KiB payload. The hash is fixed, so a corrupted transfer
// fails the test instead of silently passing.
const payload = Buffer.alloc(256 * 1024);
for (let index = 0; index < payload.length; index += 1) {
	payload[index] = index % 251;
}
const payloadHash = createHash('sha256').update(payload).digest('hex');

const here = path.dirname(fileURLToPath(import.meta.url));
const tmpRoot = path.join(here, '.tmp');
const downloadDir = path.join(tmpRoot, 'downloads');
const downloadedFile = path.join(downloadDir, 'sample.bin');

let server: Server;
let fileUrl = '';

test.beforeAll(async () => {
	await rm(tmpRoot, { recursive: true, force: true });
	await mkdir(downloadDir, { recursive: true });

	server = createServer((request, response) => {
		if (request.url === '/sample.bin') {
			response.writeHead(200, {
				'Content-Type': 'application/octet-stream',
				'Content-Length': payload.length
			});
			response.end(payload);
			return;
		}
		response.writeHead(404).end();
	});

	await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
	const address = server.address();
	if (!address || typeof address === 'string') {
		throw new Error('test server did not report a port');
	}
	fileUrl = `http://127.0.0.1:${address.port}/sample.bin`;
});

test.afterAll(async () => {
	await new Promise<void>((resolve) => server.close(() => resolve()));
	// Belt and braces: even a failed run leaves no test files behind.
	await rm(tmpRoot, { recursive: true, force: true });
});

test('UI: enable, add a download, verdict, disable, clear the list', async ({ page }) => {
	await page.goto('/');

	await expect(page.getByRole('heading', { name: 'Aus' })).toBeVisible();
	await page.getByRole('button', { name: 'Schutz einschalten' }).click();
	await expect(page.getByRole('heading', { name: 'Geschützt' })).toBeVisible({ timeout: 15_000 });

	await page.getByRole('button', { name: 'Downloads öffnen' }).click();
	const downloads = page.getByRole('dialog', { name: 'Downloads' });
	await expect(downloads).toBeVisible();
	await expect(downloads.getByText('Keine Downloads in dieser Sitzung.')).toBeVisible();

	await downloads.getByLabel('Download-URL').fill('https://example.org/sample.bin');
	await downloads.getByRole('button', { name: 'Hinzufügen' }).click();

	const item = downloads.getByTestId('download-item').first();
	await expect(item).toContainText('sample.bin');
	await expect(item).toContainText('Sauber', { timeout: 15_000 });

	await downloads.getByRole('button', { name: 'Fertig' }).click();

	await page.getByRole('button', { name: 'Schutz ausschalten' }).click();
	const summary = page.getByRole('dialog', { name: 'Sitzung beendet' });
	await expect(summary).toBeVisible({ timeout: 15_000 });
	await expect(summary.locator('li').filter({ hasText: 'Sauber' })).toContainText('1');
	await summary.getByRole('button', { name: 'Fertig' }).click();

	// Cleanup in the app: the list must be empty again.
	await page.getByRole('button', { name: 'Downloads öffnen' }).click();
	const downloadsAgain = page.getByRole('dialog', { name: 'Downloads' });
	await downloadsAgain.getByRole('button', { name: 'Liste leeren' }).click();
	await expect(downloadsAgain.getByText('Keine Downloads in dieser Sitzung.')).toBeVisible();
});

test('bulk route: enable VPN torrents, add a magnet, clear the list', async ({ page }) => {
	await page.goto('/');

	await page.getByRole('button', { name: 'Schutz einschalten' }).click();
	await expect(page.getByRole('heading', { name: 'Geschützt' })).toBeVisible({ timeout: 15_000 });

	await page.getByRole('button', { name: 'Einstellungen öffnen' }).click();
	const settings = page.getByRole('dialog', { name: 'Einstellungen' });
	await settings.getByRole('button', { name: 'Nur VPN' }).click();
	await settings.getByRole('button', { name: 'Fertig' }).click();

	await page.getByRole('button', { name: 'Downloads öffnen' }).click();
	const downloads = page.getByRole('dialog', { name: 'Downloads' });
	await downloads.getByRole('button', { name: 'Über VPN' }).click();
	await expect(
		downloads.getByText('Braucht den Dienst. Torrents laufen erst mit der VPN-Schicht.')
	).toBeVisible();

	await downloads
		.getByLabel('Download-URL')
		.fill('magnet:?dn=debian.iso&xt=urn:btih:0123456789abcdef0123456789abcdef01234567');
	await downloads.getByRole('button', { name: 'Hinzufügen' }).click();

	const item = downloads.getByTestId('download-item').first();
	await expect(item).toContainText('debian.iso');
	await expect(item).toContainText('VPN');
	await expect(item).toContainText('Sauber', { timeout: 15_000 });

	await downloads.getByRole('button', { name: 'Liste leeren' }).click();
	await expect(downloads.getByText('Keine Downloads in dieser Sitzung.')).toBeVisible();
});

test('bulk route is refused while torrents are blocked', async ({ page }) => {
	await page.goto('/');
	await page.getByRole('button', { name: 'Schutz einschalten' }).click();
	await expect(page.getByRole('heading', { name: 'Geschützt' })).toBeVisible({ timeout: 15_000 });

	await page.getByRole('button', { name: 'Downloads öffnen' }).click();
	const downloads = page.getByRole('dialog', { name: 'Downloads' });
	await downloads.getByRole('button', { name: 'Über VPN' }).click();
	await downloads
		.getByLabel('Download-URL')
		.fill('magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567');
	await downloads.getByRole('button', { name: 'Hinzufügen' }).click();

	await expect(
		downloads.getByText('Die VPN-Route ist ausgeschaltet. Aktiviere sie in den Einstellungen.')
	).toBeVisible();
	await expect(downloads.getByTestId('download-item')).toHaveCount(0);
});

test('files: download to disk, verify the hash, remove everything', async ({ request }) => {
	const response = await request.get(fileUrl);
	expect(response.ok()).toBeTruthy();

	const body = await response.body();
	expect(body.length).toBe(payload.length);

	await writeFile(downloadedFile, body);
	await writeFile(
		path.join(downloadDir, 'manifest.json'),
		`${JSON.stringify(
			{
				file: 'sample.bin',
				bytes: body.length,
				sha256: payloadHash,
				created_at: new Date().toISOString()
			},
			null,
			2
		)}\n`
	);

	expect(existsSync(downloadedFile)).toBe(true);
	const stored = await readFile(downloadedFile);
	expect(createHash('sha256').update(stored).digest('hex')).toBe(payloadHash);

	// Remove the download folder and prove it is gone.
	await rm(downloadDir, { recursive: true, force: true });
	expect(existsSync(downloadDir)).toBe(false);
	expect(existsSync(tmpRoot)).toBe(true);

	await rm(tmpRoot, { recursive: true, force: true });
	expect(existsSync(tmpRoot)).toBe(false);
});
