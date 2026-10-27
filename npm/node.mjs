// ESM entry for Node: re-exports the CommonJS wasm-bindgen build.
import wasm from './dist/node/plantuml_wasm.js';

export const render = wasm.render;
export const init_logging = wasm.init_logging;
export default wasm;
