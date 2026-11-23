import type { AppState, BannerKind } from './types';

/**
 * Banners a deliberate teardown starts with.
 *
 * A `failclosed` banner can predate the teardown (a failed enable, an earlier
 * abort). Keeping it would make the running teardown look like a failed one and
 * offer the emergency reset while Tor, VPN and the filters are still being
 * removed. A teardown that really fails re-adds the banner through the service
 * status, so nothing is hidden.
 */
export function bannersAfterTeardownStart(banners: BannerKind[]): BannerKind[] {
	return banners.filter((banner) => banner !== 'failclosed');
}

/**
 * Bookkeeping every deliberate teardown performs before it starts.
 *
 * Kept in one place so the four entry points cannot drift apart: the state moves
 * to TEARDOWN, the previous failure code is dropped (a fresh attempt is not a
 * failed one) and only the `failclosed` banner goes away. A teardown that really
 * fails reports that through its own banner or code afterwards.
 */
export function teardownStart(banners: BannerKind[]): {
	state: AppState;
	banners: BannerKind[];
	failureCode: null;
} {
	return { state: 'TEARDOWN', banners: bannersAfterTeardownStart(banners), failureCode: null };
}

/**
 * Whether a session summary may be written after a teardown that started in
 * `reached`.
 *
 * OFF has no session to report, and TEARDOWN is not a state a session reached: a
 * second panic during a hanging teardown must not overwrite the summary of the
 * session it interrupted (the service refuses the same, see
 * `service/tvpnd/src/lib.rs`).
 */
export function shouldWriteSummary(reached: AppState): boolean {
	return reached !== 'OFF' && reached !== 'TEARDOWN';
}

/**
 * Whether the UI must offer the emergency reset instead of a busy state.
 *
 * Only a teardown that carries its own evidence counts as failed: a failure
 * code or a fresh `failclosed` banner. The state alone is not enough, because
 * every user-started teardown passes through TEARDOWN.
 */
export function isFailedTeardown(
	state: AppState,
	failureCode: string | null,
	banners: BannerKind[]
): boolean {
	return state === 'TEARDOWN' && (!!failureCode || banners.includes('failclosed'));
}
