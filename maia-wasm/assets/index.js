import init from "./pkg/maia_wasm.js";

async function run() {
    const wasm = await init();
};

run();
