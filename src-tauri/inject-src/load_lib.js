import * as tauriAppsapi from 'https://cdn.jsdelivr.net/npm/@tauri-apps/api@1.6.0/index.min.js';
window.tauri_api = tauriAppsapi;
window.addEventListener('load', () => {
    import("./inject.js");
}, { once: true });

