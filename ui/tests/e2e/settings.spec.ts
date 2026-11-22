import { expect, test } from '@playwright/test';

import { APP_VERSION } from '../../src/lib/version';

test('tester diagnostics: clipboard gets version, state and settings', async ({ page, context }) => {
	await page.goto('/');
	await context.grantPermissions(['clipboard-read', 'clipboard-write'], {
		origin: 'http://localhost:5173'
	});

	await expect(page.getByRole('heading', { name: 'Aus' })).toBeVisible();
	await page.getByRole('button', { name: 'Einstellungen öffnen' }).click();

	const settings = page.getByRole('dialog', { name: 'Einstellungen' });
	await expect(settings).toBeVisible();
	await settings.getByRole('button', { name: 'Tester-Infos kopieren' }).click();
	await expect(settings.getByRole('status')).toHaveText('Kopiert.');

	const report = await page.evaluate(() => navigator.clipboard.readText());
	expect(report).toContain(`TVPN Pro ${APP_VERSION} (Alpha)`);
	expect(report).toContain('Zustand: Aus');
	expect(report).toContain('Torrents blockiert');
	expect(report).toContain('Enthält keine IP-Adressen');
	expect(report).not.toContain('http');
});
