#!/usr/bin/env node
// Rendering profile runner. See profile/README.md.
//
//   node profile/run.mjs [--label NAME] [--baseline] [--runs N] [--frames-scale X]
//                        [--filter REGEX] [--no-build] [--size WxH] [--gpu] [--out FILE]
import { execSync } from 'node:child_process';
import { createReadStream, existsSync, mkdirSync, statSync, writeFileSync } from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';
import { SCENARIOS } from './scenarios.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const PORT = 4174;
const WARMUP_FRAMES = 30;
const DT_MS = 16;

const argv = process.argv.slice(2);
const flag = (n) => argv.includes(`--${n}`);
const opt = (n, d) => (argv.includes(`--${n}`) ? argv[argv.indexOf(`--${n}`) + 1] : d);

const sh = (cmd) => execSync(cmd, { cwd: ROOT, encoding: 'utf8' }).trim();
const gitRev = (() => {
  try {
    return { rev: sh('git rev-parse --short HEAD'), dirty: sh('git status --porcelain -- backend voxel-core').length > 0 };
  } catch {
    return { rev: 'nogit', dirty: false };
  }
})();

const label = opt('label', gitRev.rev + (gitRev.dirty ? '-dirty' : ''));
const runs = Number(opt('runs', 3));
const scale = Number(opt('frames-scale', 1));
const filter = new RegExp(opt('filter', '.'));
const [vw, vh] = opt('size', '640x360').split('x').map(Number);
const out =
  opt('out', null) ?? path.join(ROOT, 'profile', flag('baseline') ? 'baselines' : 'results', `${label}.json`);

// SwiftShader is the only adapter that is reproducible across machines (same as the pixel tests).
// --gpu tries whatever the host offers instead; numbers are then only comparable on that host.
const chromeArgs = [
  '--enable-unsafe-webgpu',
  '--enable-features=Vulkan',
  // GPU timestamp queries + un-quantised timestamps (otherwise 100 us steps).
  '--enable-dawn-features=allow_unsafe_apis',
  '--disable-dawn-features=timestamp_quantization',
  ...(flag('gpu') ? ['--ignore-gpu-blocklist'] : ['--use-vulkan=swiftshader', '--use-angle=swiftshader']),
];

if (!flag('no-build')) {
  console.log('building wasm (test-hooks) + bundle...');
  sh('pnpm wasm:test && pnpm exec vite build');
}

// Static server for dist/. Cross-origin isolation (COOP/COEP) makes performance.now() tick at
// 5 us instead of 100 us: without it every sub-ms CPU timing is quantised garbage.
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.css': 'text/css', '.png': 'image/png', '.jpg': 'image/jpeg', '.json': 'application/json' };
const DIST = path.join(ROOT, 'dist');
const server = http.createServer((req, res) => {
  let p = path.normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname));
  if (p.endsWith(path.sep)) p += 'index.html';
  const file = path.join(DIST, p);
  const headers = { 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp', 'Cache-Control': 'no-store' };
  if (!file.startsWith(DIST) || !existsSync(file) || statSync(file).isDirectory()) {
    res.writeHead(404, headers).end();
    return;
  }
  res.writeHead(200, { ...headers, 'Content-Type': MIME[path.extname(file)] ?? 'application/octet-stream' });
  createReadStream(file).pipe(res);
});
await new Promise((r) => server.listen(PORT, r));
const url = `http://localhost:${PORT}`;

/** Runs inside the page. Everything but the frame loop is outside the timed region. */
async function pageRun([sc, warmup, dt]) {
  const w = window.__rwg;
  const fields = w.test_profile_fields();
  fields.push('gpu_pass', 'gpu_done');
  const frames = Object.fromEntries(fields.map((f) => [f, []]));
  const settle = w.test_settle(5000, dt);
  if (settle >= 5000) throw new Error('world did not settle');
  for (const [k, v] of Object.entries(sc.toggles)) w.test_set_toggle(k, v);

  const pose = (i) => {
    if (sc.pose?.fixed) return sc.pose.fixed;
    if (sc.pose?.orbit) {
      const a = (2 * Math.PI * i) / sc.frames;
      return [[0, 1, 0], [sc.pose.orbit.radius * Math.cos(a), sc.pose.orbit.height, sc.pose.orbit.radius * Math.sin(a)]];
    }
    return null;
  };
  const total = warmup + sc.frames;
  for (let i = 0; i < total; i++) {
    const p = pose(Math.max(0, i - warmup));
    if (p) w.test_set_camera(p[0], p[1]);
    if (sc.move) w.test_move_player(sc.move);
    const row = Array.from(w.test_profile_step(dt));
    // one frame in flight at a time: GPU numbers must not include a backlog
    await w.test_gpu_idle();
    row.push(...w.test_gpu_collect());
    if (i >= warmup) fields.forEach((f, j) => frames[f].push(Number.isNaN(row[j]) ? null : Math.round(row[j] * 1e4) / 1e4));
  }
  return { settleFrames: settle, frames };
}

async function runScenario(browser, sc, frames) {
  const ctx = await browser.newContext({ viewport: { width: vw, height: vh }, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => console.log('  pageerror:', e.message));
  await page.addInitScript(() => (window.__RWG_TEST__ = true));
  await page.goto(url);
  await page.waitForFunction(() => window.__rwg?.test_start);
  // egui off unless the scenario turns it on: isolates the world renderer.
  await page.evaluate(() => window.__rwg.test_start('default_spawn'));
  const res = await page.evaluate(pageRun, [{ ...sc, frames }, WARMUP_FRAMES, DT_MS]);
  // after the frames: the canvas only gets its real size on the first render
  const info = JSON.parse(await page.evaluate(() => window.__rwg.test_profile_info()));
  await ctx.close();
  return { info, ...res };
}

try {
  const browser = await chromium.launch({ args: chromeArgs });
  const scenarios = SCENARIOS.filter((s) => filter.test(s.name));
  const result = {
    meta: {
      label,
      git: gitRev.rev,
      dirty: gitRev.dirty,
      date: new Date().toISOString(),
      runs,
      viewport: [vw, vh],
      warmupFrames: WARMUP_FRAMES,
      dtMs: DT_MS,
      mode: flag('gpu') ? 'gpu' : 'swiftshader',
      host: { cpu: os.cpus()[0]?.model, cores: os.cpus().length, load1: os.loadavg()[0], node: process.version },
      browser: browser.version(),
      env: null,
    },
    scenarios: Object.fromEntries(scenarios.map((s) => [s.name, { desc: s.desc, runs: [] }])),
  };
  // Interleave: run r of every scenario before run r+1, so slow drift (thermals, background
  // load) hits all scenarios alike instead of the last one.
  for (let r = 0; r < runs; r++) {
    for (const sc of scenarios) {
      const frames = Math.max(10, Math.round(sc.frames * scale));
      process.stdout.write(`run ${r + 1}/${runs}  ${sc.name} (${frames} frames)... `);
      const t = Date.now();
      const res = await runScenario(browser, sc, frames);
      result.meta.env ??= res.info;
      result.scenarios[sc.name].runs.push({ settleFrames: res.settleFrames, frames: res.frames });
      console.log(`${((Date.now() - t) / 1000).toFixed(1)}s`);
    }
  }
  await browser.close();
  mkdirSync(path.dirname(out), { recursive: true });
  writeFileSync(out, JSON.stringify(result));
  console.log(`\nwrote ${path.relative(ROOT, out)}`);
  const e = result.meta.env;
  console.log(`adapter: ${e.adapter}\ngpu timestamps: ${e.gpu_timestamps}  canvas: ${e.canvas}  wasm opt: ${e.opt_level}`);
  console.log(`compare: node profile/compare.mjs profile/baselines/<base>.json ${path.relative(ROOT, out)}`);
} finally {
  server.close();
}
