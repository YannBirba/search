import { useRegisterSW } from "virtual:pwa-register/react";

function PWABadge() {
	const period = 0;

	const {
		offlineReady: [offlineReady, setOfflineReady],
		needRefresh: [needRefresh, setNeedRefresh],
		updateServiceWorker,
	} = useRegisterSW({
		onRegisteredSW(swUrl, r) {
			if (period <= 0) return;
			if (r?.active?.state === "activated") {
				registerPeriodicSync(period, swUrl, r);
			} else if (r?.installing) {
				r.installing.addEventListener("statechange", (e) => {
					const sw = e.target as ServiceWorker;
					if (sw.state === "activated") registerPeriodicSync(period, swUrl, r);
				});
			}
		},
	});

	function close() {
		setOfflineReady(false);
		setNeedRefresh(false);
	}

	return (
		<div role="alert" aria-labelledby="toast-message">
			{(offlineReady || needRefresh) && (
				<div
					style={{
						position: "fixed",
						bottom: "1rem",
						right: "1rem",
						left: "1rem",
						backgroundColor: "rgba(0, 0, 0, 0.8)",
						color: "white",
						padding: "1rem",
						textAlign: "center",
						borderRadius: "0.5rem",
						zIndex: 1000,
						backdropFilter: "blur(30px)",
					}}
				>
					<div>
						{offlineReady ? (
							<span id="toast-message">App ready to work offline</span>
						) : (
							<span id="toast-message">
								New content available, click on reload button to update.
							</span>
						)}
					</div>
					<div>
						{needRefresh && (
							<button type="button" onClick={() => updateServiceWorker(true)}>
								Reload
							</button>
						)}
						<button type="button" onClick={() => close()}>
							Close
						</button>
					</div>
				</div>
			)}
		</div>
	);
}

export default PWABadge;

/**
 * This function will register a periodic sync check every hour, you can modify the interval as needed.
 */
function registerPeriodicSync(
	period: number,
	swUrl: string,
	r: ServiceWorkerRegistration,
) {
	if (period <= 0) return;

	setInterval(async () => {
		if ("onLine" in navigator && !navigator.onLine) return;

		const resp = await fetch(swUrl, {
			cache: "no-store",
			headers: {
				cache: "no-store",
				"cache-control": "no-cache",
			},
		});

		if (resp?.status === 200) await r.update();
	}, period);
}
