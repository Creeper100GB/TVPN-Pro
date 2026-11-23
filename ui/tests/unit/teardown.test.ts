import { describe, test } from 'node:test';
import assert from 'node:assert/strict';

import {
	bannersAfterTeardownStart,
	isFailedTeardown,
	shouldWriteSummary,
	teardownStart
} from '../../src/lib/teardown';

describe('teardown start drops a stale fail-closed banner', () => {
	test('a panic from FAILCLOSED is not labelled as an unfinished teardown', () => {
		// FAILCLOSED left a `failclosed` banner behind (failed enable, earlier abort)
		// next to `no_filter`, which still describes the missing filter.
		const next = teardownStart(['failclosed', 'no_filter']);

		assert.equal(next.state, 'TEARDOWN');
		// Only the stale banner goes; the honest one survives.
		assert.deepEqual(next.banners, ['no_filter']);
		assert.equal(next.failureCode, null);
		// No emergency-reset offer while the teardown is still running.
		assert.equal(isFailedTeardown(next.state, next.failureCode, next.banners), false);
	});

	test('a genuinely failed teardown keeps its reset affordance', () => {
		const failedWithCode = isFailedTeardown('TEARDOWN', 'E_FIREWALL_ARM_FAILED', []);
		// The rule drops the stale banner, so a service-reported failure has to
		// arrive through a fresh banner or code, which this asserts stays visible.
		const failedWithBanner = isFailedTeardown('TEARDOWN', null, ['failclosed']);

		assert.equal(failedWithCode, true);
		assert.equal(failedWithBanner, true);
	});

	test('the rule only touches teardowns', () => {
		assert.equal(isFailedTeardown('FAILCLOSED', null, ['failclosed']), false);
		assert.equal(isFailedTeardown('READY', null, []), false);
		assert.deepEqual(bannersAfterTeardownStart(['degraded', 'tor_only']), ['degraded', 'tor_only']);
	});
});

describe('a teardown never reports TEARDOWN as the state a session reached', () => {
	test('a session that ended in protection reports its state', () => {
		for (const reached of ['READY', 'FAILCLOSED', 'DEGRADED'] as const) {
			assert.equal(shouldWriteSummary(reached), true, `${reached} is a reached state`);
		}
	});

	test('OFF has no session and TEARDOWN is not a reached state', () => {
		assert.equal(shouldWriteSummary('OFF'), false);
		assert.equal(shouldWriteSummary('TEARDOWN'), false);
	});
});
