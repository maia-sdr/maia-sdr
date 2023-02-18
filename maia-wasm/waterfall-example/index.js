import init, { make_waterfall } from "./pkg/waterfall_example.js";

async function run() {
    await init();
    make_waterfall("waterfall");
};

run();
