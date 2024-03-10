import './style.css'
// import typescriptLogo from './typescript.svg'
// import viteLogo from '/vite.svg'

import init, { } from '../backend/pkg';

init().then(() => {
  console.log('init wasm-pack');
});
