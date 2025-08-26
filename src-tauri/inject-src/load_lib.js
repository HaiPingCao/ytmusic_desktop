import * as tauriAppsapi from 'https://cdn.jsdelivr.net/npm/@tauri-apps/api@1.6.0/index.min.js';
import "https://cdn.jsdelivr.net/npm/dompurify@3.2.6/dist/purify.min.js";

window.tauri_api = tauriAppsapi;
window.addEventListener('load', () => {
    import("./inject.js");
    import("./plugin_inject.js");
}, {once: true});

let wakeLock = null;

async function requestWakeLock() {
    let window_tauri = await window.tauri_api.window.getCurrent();
    const isFullscreen = await window_tauri.isFullscreen();
    console.log("Requesting wake lock...", wakeLock, isFullscreen);
    if (isFullscreen) {
        try {
            wakeLock = await navigator.wakeLock.request('screen');
        } catch (err) {
            console.error(`${err.name}, ${err.message}`);
        }
    } else {
        if (wakeLock !== null) {
            await wakeLock.release();
            wakeLock = null;
        }
    }
}

document.addEventListener('fullscreenchange', async () => {
    let window_tauri = await window.tauri_api.window.getCurrent();
    window_tauri.setFullscreen(Boolean(document.fullscreenElement));
    await requestWakeLock();
});


window.addEventListener('keydown', async (event) => {
    if (event.key === 'F11') {
        let window_tauri = await window.tauri_api.window.getCurrent();
        let isFullscreen = await window_tauri.isFullscreen();
        window_tauri.setFullscreen(!isFullscreen);
        event.preventDefault();
        await requestWakeLock();
    }
});

