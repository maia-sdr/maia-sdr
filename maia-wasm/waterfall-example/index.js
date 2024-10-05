import init, { make_waterfall, make_waterfall_with_ui } from "./pkg/waterfall_example.js";

async function run() {
    await init();
    
    // use this for a waterfall with no UI form elements
    // make_waterfall("waterfall");
    
    // use this for a waterfall with UI form elements
    make_waterfall_with_ui("waterfall");
};

run();
