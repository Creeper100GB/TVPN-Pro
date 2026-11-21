import { chromium } from 'playwright';

const browser = await chromium.connectOverCDP('http://127.0.0.1:9222', { timeout: 60_000 });
const page = browser
	.contexts()
	.flatMap((context) => context.pages())
	.find((candidate) => candidate.url().includes('localhost:5173'));

if (!page) throw new Error('no page');

const result = await page.evaluate(() => ({
	full: document.documentElement.classList.contains('window-full'),
	radius: getComputedStyle(document.querySelector('.window')).borderRadius
}));

console.log(JSON.stringify(result));
process.exit(result.full && result.radius === '0px' ? 0 : 1);
