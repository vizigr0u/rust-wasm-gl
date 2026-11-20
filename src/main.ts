import './style.css'
// import typescriptLogo from './typescript.svg'
// import viteLogo from '/vite.svg'

import init, * as wasm from '../backend/pkg';
import { installDebugOverlay } from './debug-overlay';

// Inert until Rust calls window.__rwgDebug. Test mode always overlays (canvas size stays fixed).
const testMode = !!(window as any).__RWG_TEST__;
installDebugOverlay((name, on) => (wasm as any).debug_set_toggle?.(name, on), {
  docked: !testMode && new URLSearchParams(location.search).has('dock'),
});

// Meshing runs in a worker; the wasm side picks it up from window.__meshWorker.
// Test mode meshes inline (deterministic frame counts for the pixel specs).
if (!(window as any).__RWG_TEST__) {
  (window as any).__meshWorker = new Worker(new URL('./mesh-worker.ts', import.meta.url), {
    type: 'module',
  });
  // ?trace: the worker emits its own DevTools timing entries.
  if (new URLSearchParams(location.search).has('trace')) {
    (window as any).__meshWorker.postMessage({ trace: true });
  }
}

init().then(() => {
  console.log('init wasm-pack');
  // e2e specs drive the game through the exported test_* functions.
  if ((window as any).__RWG_TEST__) (window as any).__rwg = wasm;
});
