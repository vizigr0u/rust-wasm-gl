import './style.css'
// import typescriptLogo from './typescript.svg'
// import viteLogo from '/vite.svg'

import init, * as wasm from '../backend/pkg';

init().then(() => {
  console.log('init wasm-pack');
  // e2e specs drive the game through the exported test_* functions.
  if ((window as any).__RWG_TEST__) (window as any).__rwg = wasm;
});
