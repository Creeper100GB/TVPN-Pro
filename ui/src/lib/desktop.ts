// Tauri only: the browser preview has no window API. Guarded so the same
// component tree runs in both without a second code path.

export const isDesktop =
	typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export type WindowAction = 'close' | 'minimize' | 'maximize';

export async function windowAction(action: WindowAction): Promise<boolean> {
	if (!isDesktop) return false;

	const { getCurrentWindow } = await import('@tauri-apps/api/window');
	const windowHandle = getCurrentWindow();

	switch (action) {
		case 'close':
			await windowHandle.close();
			break;
		case 'minimize':
			await windowHandle.minimize();
			break;
		case 'maximize':
			await windowHandle.toggleMaximize();
			break;
	}

	return true;
}
