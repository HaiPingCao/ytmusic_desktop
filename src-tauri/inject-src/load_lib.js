import * as tauriAppsapi from 'https://cdn.jsdelivr.net/npm/@tauri-apps/api@1.6.0/index.min.js';
import "https://cdn.jsdelivr.net/npm/dompurify@3.2.6/dist/purify.min.js";
window.tauri_api = tauriAppsapi;
window.addEventListener('load', () => {
    import("./inject.js");
    import("./plugin_inject.js");
}, { once: true });

