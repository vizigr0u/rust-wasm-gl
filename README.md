## What is this?

trying to make a little game in Rust WASM + WebGL2.

As of initial github publish / first commits : Getting WebGL2 boiler plate + initial game structs.

![image](https://github.com/vizigr0u/rust-wasm-gl/assets/1981001/75080744-f8b1-42a1-a480-cf855dabb6b2)

for now I don't want to bother with JS/WASM interaction too much so everything is handled on the WASM (Rust) side.
Not sure I'll ever change this.

Web page : just a canvas
uses vite + Typescript (for some reason all rust WASM doc uses webpack but I am stubborn I wanted Vite)

Game (in the backend directory): Rust compiled to WASM
uses glam for maths and web-sys for JS bindings

## Getting Started

```bash
git clone https://github.com/vizigr0u/rust-wasm-gl
yarn install

# compile Rust into WASM
yarn wasm

# start web server
yarn dev
```
