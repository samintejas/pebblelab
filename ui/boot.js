// the hub (pebblelab.in) is served the same page; it is not fin, so it must not install as fin
const h = location.hostname;
if (h === 'pebblelab.in' || h === 'www.pebblelab.in' || h.startsWith('hub.')) document.querySelector('link[rel=manifest]')?.remove();
import init from '/pebblelab-ui.js?v=__V__';
init({ module_or_path: '/pebblelab-ui_bg.wasm?v=__V__' });
