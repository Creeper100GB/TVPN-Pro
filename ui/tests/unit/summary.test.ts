import { describe, test } from 'node:test';
import assert from 'node:assert/strict';

import { summarizeDownloadList } from '../../src/lib/summary';
import type { DownloadItem } from '../../src/lib/types';

function item(verdict: DownloadItem['verdict']): DownloadItem {
	return {
		id: verdict ?? 'running',
		label: 'file.bin',
		phase: verdict === null ? 'active' : 'done',
		progress: verdict === null ? 40 : 100,
		verdict,
		route: 'tor',
		source: 'https'
	};
}

describe('the session summary counts every verdict in a bucket', () => {
	test('quarantine_failed counts as failed, not nowhere', () => {
		// The threat was detected but could not be moved: the file was deleted,
		// so the download delivered nothing usable.
		const counts = summarizeDownloadList([item('quarantine_failed')]);

		assert.deepEqual(counts, { clean: 0, warned: 0, quarantined: 0, failed: 1 });
	});

	test('the remaining verdicts land in their contract buckets', () => {
		const counts = summarizeDownloadList([
			item('clean'),
			item('warned'),
			item('unknown'),
			item('threat'),
			item('quarantined'),
			item('failed'),
			item(null)
		]);

		assert.deepEqual(counts, { clean: 1, warned: 3, quarantined: 1, failed: 1 });
	});
});
