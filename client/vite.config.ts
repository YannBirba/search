import react from "@vitejs/plugin-react-swc";
import { defineConfig } from "vite";
import { VitePWA } from "vite-plugin-pwa";

// https://vite.dev/config/
export default defineConfig({
	plugins: [
		react(),
		VitePWA({
			registerType: "prompt",
			strategies: "generateSW",
			injectRegister: false,
			pwaAssets: {
				disabled: false,
				config: true,
				htmlPreset: "2023",
				overrideManifestIcons: true,
			},
			manifest: {
				name: "Search",
				short_name: "Search",
				description: "Meta search engine",
				theme_color: "#084887",
				icons: [
					{
						src: "pwa-64x64.png",
						sizes: "64x64",
						type: "image/png",
					},
					{
						src: "pwa-192x192.png",
						sizes: "192x192",
						type: "image/png",
					},
					{
						src: "pwa-512x512.png",
						sizes: "512x512",
						type: "image/png",
					},
					{
						src: "maskable-icon-512x512.png",
						sizes: "512x512",
						type: "image/png",
						purpose: "maskable",
					},
					{
						src: "apple-touch-icon-180x180.png",
						sizes: "180x180",
						type: "image/png",
					},
					{
						src: "favicon.ico",
						sizes: "64x64",
						type: "image/x-icon",
					},
				],
				display: "standalone",
				display_override: ["standalone", "minimal-ui", "browser", "fullscreen"],
				orientation: "portrait",
			},
			workbox: {
				cleanupOutdatedCaches: true,
				clientsClaim: true,
				globPatterns: ["**/*.{js,css,html,svg,png,svg,ico}"],
				runtimeCaching: [
					{
						urlPattern: /^https:\/\/fonts\.bunny\.net\/.*/i,
						handler: "CacheFirst",
						options: {
							cacheName: "bunny-fonts-cache",
							expiration: {
								maxEntries: 10,
								maxAgeSeconds: 60 * 60 * 24 * 365, // <== 365 days
							},
							cacheableResponse: {
								statuses: [0, 200],
							},
						},
					},
				],
			},
			injectManifest: {
				globPatterns: ["**/*.{js,css,html,svg,png,svg,ico}"],
			},
			devOptions: {
				enabled: false,
				navigateFallback: "index.html",
				suppressWarnings: true,
				type: "module",
			},
		}),
	],
});
