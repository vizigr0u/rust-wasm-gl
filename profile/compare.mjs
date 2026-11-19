#!/usr/bin/env node
// Compare two profile runs (or summarise one). See profile/README.md.
//
//   node profile/compare.mjs BASE.json HEAD.json [--fail-on-regress] [--min-effect PCT]
//                                                [--alpha P] [--verbose]
//   node profile/compare.mjs RUN.json                      (summary only)
import { readFileSync } from 'node:fs';

const argv = process.argv.slice(2);
const files = argv.filter((a, i) => !a.startsWith('--') && !(i > 0 && ['--min-effect', '--alpha'].includes(argv[i - 1])));
const opt = (n, d) => (argv.includes(`--${n}`) ? Number(argv[argv.indexOf(`--${n}`) + 1]) : d);
const MIN_EFFECT = opt('min-effect', 3) / 100; // ignore shifts smaller than this, noise or not
const ALPHA = opt('alpha', 0.01);
const VERBOSE = argv.includes('--verbose');
const FAIL = argv.includes('--fail-on-regress');
const ABS_FLOOR_MS = 0.02; // below this both medians are timer noise

const TIMES = ['frame', 'script', 'upload', 'ui', 'prepare', 'acquire', 'pass', 'egui', 'submit', 'gpu_pass', 'gpu_done'];
const HEADLINE = new Set(['frame', 'script', 'prepare', 'pass', 'submit', 'gpu_pass', 'gpu_done']);
const COUNTERS = ['draw_calls', 'draw_entries_raw', 'chunks_visible', 'chunks_culled', 'quads_drawn', 'upload_bytes', 'chunk_buffers', 'arena_used', 'arena_largest_free', 'wasm_mem_mb'];

const load = (f) => JSON.parse(readFileSync(f, 'utf8'));
const finite = (a) => a.filter((x) => x !== null && Number.isFinite(x));
const sorted = (a) => [...a].sort((x, y) => x - y);
const q = (s, p) => (s.length ? s[Math.min(s.length - 1, Math.floor(p * s.length))] : NaN);
const median = (a) => q(sorted(a), 0.5);
const mean = (a) => a.reduce((x, y) => x + y, 0) / a.length;
const pct = (a, b) => (a === 0 ? (b === 0 ? 0 : Infinity) : (b - a) / a);
const fmt = (x, d = 3) => (Number.isFinite(x) ? x.toFixed(d) : '-');
const fmtPct = (x) => (Number.isFinite(x) ? `${x >= 0 ? '+' : ''}${(x * 100).toFixed(1)}%` : '-');

const pooled = (sc, f) => finite(sc.runs.flatMap((r) => r.frames[f] ?? []));
const perRun = (sc, f) => sc.runs.map((r) => finite(r.frames[f] ?? [])).filter((a) => a.length);

/** Mann-Whitney U, normal approximation with tie correction. Two-sided p. */
function mannWhitneyP(a, b) {
  const n1 = a.length, n2 = b.length;
  if (n1 < 8 || n2 < 8) return 1;
  const all = [...a.map((v) => [v, 0]), ...b.map((v) => [v, 1])].sort((x, y) => x[0] - y[0]);
  const ranks = new Array(all.length);
  let tie = 0;
  for (let i = 0; i < all.length; ) {
    let j = i;
    while (j + 1 < all.length && all[j + 1][0] === all[i][0]) j++;
    const r = (i + j) / 2 + 1;
    for (let k = i; k <= j; k++) ranks[k] = r;
    const t = j - i + 1;
    tie += t ** 3 - t;
    i = j + 1;
  }
  let r1 = 0;
  all.forEach((x, i) => x[1] === 0 && (r1 += ranks[i]));
  const u = r1 - (n1 * (n1 + 1)) / 2;
  const n = n1 + n2;
  const sd = Math.sqrt(((n1 * n2) / 12) * (n + 1 - tie / (n * (n - 1))));
  if (sd === 0) return 1;
  const z = (Math.abs(u - (n1 * n2) / 2) - 0.5) / sd;
  return erfc(z / Math.SQRT2);
}
function erfc(x) {
  const t = 1 / (1 + 0.3275911 * x);
  const y = ((((1.061405429 * t - 1.453152027) * t + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t;
  return y * Math.exp(-x * x);
}

/** Relative spread of per-run medians: how much repeated runs of the same build disagree. */
function noise(sc, f) {
  const m = perRun(sc, f).map(median);
  if (m.length < 2) return 0;
  const mid = median(m);
  return mid > 0 ? (Math.max(...m) - Math.min(...m)) / mid : 0;
}

function summary(name, sc) {
  console.log(`\n### ${name}  (${sc.desc})`);
  console.log('| metric | median | mean | p95 | max | run spread |');
  console.log('|---|---:|---:|---:|---:|---:|');
  for (const f of TIMES) {
    const v = pooled(sc, f);
    if (!v.length) continue;
    const s = sorted(v);
    console.log(`| ${f} | ${fmt(q(s, 0.5))} | ${fmt(mean(v))} | ${fmt(q(s, 0.95))} | ${fmt(s.at(-1))} | ${fmtPct(noise(sc, f))} |`);
  }
  console.log(`counters (mean): ` + COUNTERS.map((f) => `${f}=${fmt(mean(pooled(sc, f)), 1)}`).join('  '));
}

function compareScenario(name, a, b) {
  console.log(`\n### ${name}  (${b.desc})`);
  console.log('| metric | base med | head med | Δ | base p95 | head p95 | p | noise | verdict |');
  console.log('|---|---:|---:|---:|---:|---:|---:|---:|---|');
  let regress = 0, improve = 0;
  for (const f of TIMES) {
    const va = pooled(a, f), vb = pooled(b, f);
    if (va.length < 8 || vb.length < 8) continue; // e.g. GPU columns absent on one side
    const [ma, mb] = [median(va), median(vb)];
    const d = pct(ma, mb);
    const p = mannWhitneyP(va, vb);
    const nz = Math.max(noise(a, f), noise(b, f));
    const tiny = Math.max(ma, mb) < ABS_FLOOR_MS;
    const thr = Math.max(MIN_EFFECT, nz);
    let verdict = 'same';
    if (!tiny && p < ALPHA && Math.abs(d) > thr) verdict = d > 0 ? 'SLOWER' : 'faster';
    if (verdict === 'SLOWER' && HEADLINE.has(f)) regress++;
    if (verdict === 'faster' && HEADLINE.has(f)) improve++;
    if (!VERBOSE && verdict === 'same' && !['frame', 'gpu_pass'].includes(f)) continue;
    console.log(`| ${f} | ${fmt(ma)} | ${fmt(mb)} | ${fmtPct(d)} | ${fmt(q(sorted(va), 0.95))} | ${fmt(q(sorted(vb), 0.95))} | ${p.toExponential(1)} | ${fmtPct(nz)} | ${verdict} |`);
  }
  const changed = [];
  for (const f of COUNTERS) {
    const [va, vb] = [pooled(a, f), pooled(b, f)];
    if (!va.length || !vb.length) continue;
    const [x, y] = f === 'wasm_mem_mb' ? [Math.max(...va), Math.max(...vb)] : [mean(va), mean(vb)];
    const rel = pct(x, y);
    if (VERBOSE || Math.abs(rel) > 1e-9) changed.push(`${f}: ${fmt(x, 1)} -> ${fmt(y, 1)} (${fmtPct(rel)})`);
  }
  console.log(changed.length ? `\ncounters changed: ${changed.join('; ')}` : '\ncounters identical');
  return { regress, improve };
}

if (files.length === 1) {
  const r = load(files[0]);
  console.log(`# ${r.meta.label}  ${r.meta.date}  ${r.meta.mode}  runs=${r.meta.runs}  ${r.meta.env?.adapter ?? ''}`);
  for (const [n, sc] of Object.entries(r.scenarios)) summary(n, sc);
  process.exit(0);
}
if (files.length !== 2) {
  console.error('usage: compare.mjs BASE.json [HEAD.json] [--fail-on-regress] [--min-effect PCT] [--alpha P] [--verbose]');
  process.exit(2);
}
const [A, B] = files.map(load);
console.log(`# ${A.meta.label} (base)  vs  ${B.meta.label} (head)`);
console.log(`base: ${A.meta.date} ${A.meta.mode} runs=${A.meta.runs} dirty=${A.meta.dirty}\nhead: ${B.meta.date} ${B.meta.mode} runs=${B.meta.runs} dirty=${B.meta.dirty}`);
const warn = [];
if (A.meta.mode !== B.meta.mode || A.meta.env?.adapter !== B.meta.env?.adapter) warn.push('different adapter/mode: GPU timings are not comparable');
if (A.meta.host?.cpu !== B.meta.host?.cpu) warn.push('different CPU: CPU timings are not comparable');
if (JSON.stringify(A.meta.viewport) !== JSON.stringify(B.meta.viewport)) warn.push('different viewport');
if (A.meta.env?.opt_level !== B.meta.env?.opt_level) warn.push('different wasm profile');
if (warn.length) console.log(`\n**WARNING:** ${warn.join('; ')}`);
console.log(`\nverdict needs p < ${ALPHA} and |Δ| > max(${MIN_EFFECT * 100}%, run-to-run noise). Times in ms.`);

let regress = 0, improve = 0;
for (const [n, sb] of Object.entries(B.scenarios)) {
  const sa = A.scenarios[n];
  if (!sa) {
    console.log(`\n### ${n}: not in base, skipped`);
    continue;
  }
  const r = compareScenario(n, sa, sb);
  regress += r.regress;
  improve += r.improve;
}
for (const n of Object.keys(A.scenarios)) if (!B.scenarios[n]) console.log(`\n### ${n}: only in base`);
console.log(`\n**summary:** ${regress} headline regression(s), ${improve} headline improvement(s)`);
if (FAIL && regress) process.exit(1);
