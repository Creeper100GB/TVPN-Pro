import type { DownloadItem } from './types';

/**
 * Summary buckets from `contracts/ipc-v0.md` section 6.4, counted from the
 * download list.
 *
 * Every verdict a download can end in has to land in one bucket: a count that
 * falls through the switch would hide a judged download from the session
 * summary.
 */
export function summarizeDownloadList(items: DownloadItem[]) {
	let clean = 0;
	let warned = 0;
	let quarantined = 0;
	let failed = 0;

	for (const item of items) {
		switch (item.verdict) {
			case 'clean':
				clean += 1;
				break;
			case 'warned':
				// "Unklar" and "Fund" have no own bucket in the summary. Counting
				// them as warnings keeps the summary from hiding judged downloads;
				// the shell normally turns a fund into quarantined or failed.
			case 'unknown':
			case 'threat':
				warned += 1;
				break;
			case 'quarantined':
				quarantined += 1;
				break;
			// A detected threat that could not be moved out of the download folder
			// delivered no usable file either: the file is deleted, not delivered.
			case 'quarantine_failed':
			case 'failed':
				failed += 1;
				break;
			case null:
				break;
		}
	}

	return { clean, warned, quarantined, failed };
}
