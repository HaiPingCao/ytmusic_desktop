(async () => {
    let { invoke } = window.tauri_api;
    let plugins = await invoke("get_plugin_list");
    console.log("Plugins to load:", plugins);
    for (let plugin of plugins) {
        console.log(`Loading plugin: ${plugin}`);
        if (plugin.endsWith('.js')) {
            try {
                let wait = new Promise((resolve, reject) => {
                    let script = document.createElement('script');
                    script.src = `https://plugin.localhost/${plugin}`;
                    script.async = true;
                    script.addEventListener('load', resolve);
                    script.addEventListener('error', reject)
                    document.head.appendChild(script);
                });
                await wait;
                console.log(`Plugin ${plugin} loaded successfully.`);
            } catch (error) {
                console.error(`Failed to load plugin ${plugin}:`, error);
            }
        } else if (plugin.endsWith('.css')) {
            let link = document.createElement('link');
            link.rel = 'stylesheet';
            link.href = `https://plugin.localhost/${plugin}`;
            link.onload = () => {
                console.log(`Stylesheet ${plugin} loaded successfully.`);
            };
            link.onerror = () => {
                console.error(`Failed to load stylesheet ${plugin}.`);
            };
            document.head.appendChild(link);
            console.log(`Stylesheet ${plugin} loaded successfully.`);
        }

    }
})();
