import { defineConfig, devices } from '@playwright/test';

// End-to-end tests run against the dev server (the same code path as the Tauri
// WebView loads). The production build is checked separately by `bun run build`.
export default defineConfig({
	testDir: './tests/e2e',
	fullyParallel: false,
	workers: 1,
	timeout: 60_000,
	expect: { timeout: 10_000 },
	reporter: [['list']],
	use: {
		baseURL: 'http://localhost:5173',
		viewport: { width: 1120, height: 780 },
		trace: 'retain-on-failure',
		video: 'off',
		colorScheme: 'light'
	},
	projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
	webServer: {
		command: 'bun run dev --port 5173',
		url: 'http://localhost:5173',
		reuseExistingServer: !process.env.CI,
		timeout: 120_000
	}
});
