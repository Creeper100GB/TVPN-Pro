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

test('settings: country and portal timeout go through the store', async ({ page, context }) => {
	await page.goto('/');
	await context.grantPermissions(['clipboard-read', 'clipboard-write'], {
		origin: 'http://localhost:5173'
	});

	await page.getByRole('button', { name: 'Einstellungen öffnen' }).click();
	const settings = page.getByRole('dialog', { name: 'Einstellungen' });
	await expect(settings).toBeVisible();

	const country = settings.getByLabel('VPN-Land');
	await country.selectOption('ch');

	const slider = settings.getByLabel('Portal-Zeitfenster in Sekunden');
	await slider.focus();
	for (let press = 0; press < 4; press += 1) await page.keyboard.press('ArrowRight');
	await expect(settings.getByText('5 min')).toBeVisible();

	// Close and reopen: both values come back from the shared store, not from
	// the DOM of a discarded dialog.
	await page.keyboard.press('Escape');
	await expect(settings).toBeHidden();
	await page.getByRole('button', { name: 'Einstellungen öffnen' }).click();
	await expect(country).toHaveValue('ch');
	await expect(settings.getByText('5 min')).toBeVisible();

	// The dialog DOM can be retained by the sheet; the diagnostics report is
	// built from the store, so it proves the values were persisted.
	await settings.getByRole('button', { name: 'Tester-Infos kopieren' }).click();
	await expect(settings.getByRole('status')).toHaveText('Kopiert.');
	const report = await page.evaluate(() => navigator.clipboard.readText());
	expect(report).toContain('Land Schweiz');
	expect(report).toContain('Portal 300s');
});

test('panic from OFF ends off without a session summary', async ({ page }) => {
	await page.goto('/');
	await expect(page.locator('#state-title')).toHaveText('Aus');

	await page.getByRole('button', { name: 'Einstellungen öffnen' }).click();
	const settings = page.getByRole('dialog', { name: 'Einstellungen' });
	await expect(settings).toBeVisible();

	// From OFF the sheet is the only place with a panic button.
	await settings.getByRole('button', { name: 'Sofort beenden' }).click();
	await expect(page.locator('#state-title')).toHaveText('Wird beendet');

	// A panic from OFF has no session to report: the state returns to OFF and the
	// summary dialog stays closed.
	await expect(page.locator('#state-title')).toHaveText('Aus', { timeout: 15_000 });
	await expect(page.locator('dialog[aria-label="Sitzung beendet"][open]')).toHaveCount(0);
});

test('a normal panic is not labelled as an unfinished teardown', async ({ page }) => {
	await page.goto('/');
	await page.getByRole('button', { name: 'Schutz einschalten' }).click();
	await expect(page.locator('#state-title')).toHaveText('Tor aktiv', { timeout: 15_000 });

	await page.getByRole('button', { name: 'Panik: Schutz beenden, Netz öffnen' }).click();

	// A teardown the user started is not a failure: the label stays "Wird
	// beendet" and the reset label is reserved for a failed teardown.
	await expect(page.locator('#state-title')).toHaveText('Wird beendet');
	await expect(page.getByText('Abbau unvollständig')).toHaveCount(0);
	await expect(page.getByRole('button', { name: 'Notfall-Reset starten' })).toHaveCount(0);

	// The panic from READY still opens the summary and ends in OFF.
	const summary = page.getByRole('dialog', { name: 'Sitzung beendet' });
	await expect(summary).toBeVisible({ timeout: 15_000 });
	await summary.getByRole('button', { name: 'Fertig' }).click();
	await expect(page.locator('#state-title')).toHaveText('Aus');
});
