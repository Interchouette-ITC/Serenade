const bootstrap = document.querySelector(
    'script[data-serenade-universal-bootstrap="true"]',
);

if (bootstrap && window.location.pathname === "/cms/hello") {
    const jsUrl = bootstrap.dataset.wasmJs;
    const wasmUrl = bootstrap.dataset.wasmUrl;

    if (!jsUrl || !wasmUrl) {
        throw new Error("Universal WASM bootstrap is missing bundle URLs.");
    }

    const { default: init, ...bindings } = await import(jsUrl);
    const wasm = bootstrap.dataset.initWithObject === "true"
        ? await init({ module_or_path: wasmUrl })
        : await init();

    if (bootstrap.dataset.importBindings === "true") {
        const bindingsName = bootstrap.dataset.importBindingsName;
        if (!bindingsName) {
            throw new Error("Universal WASM bootstrap is missing the bindings name.");
        }
        window[bindingsName] = bindings;
    }

    window.dispatchEvent(
        new CustomEvent("TrunkApplicationStarted", { detail: { wasm } }),
    );
}
