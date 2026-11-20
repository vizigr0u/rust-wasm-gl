// DOM debug overlay. Rust pushes state in via window.__rwgDebug (only when dirty);
// user actions go back through `setToggle` (wasm debug_set_toggle).
//
// HMR: Rust caches window.__rwgDebug and its functions once, and skips values it thinks
// are unchanged. So __rwgDebug is a stable facade (created once, never replaced) that
// records state and forwards to `impl`. On hot swap the new module rebuilds the DOM,
// swaps `impl` and replays `state`. Keep facade methods trivial: if they change, reload.

const COLORS = ['#ff6b81', '#3cb44b', '#6f8fff', '#f58231', '#c07cff', '#42d4f4', '#f032e6', '#bfef45'];

const TOGGLES: [string, string][] = [
  ['frustum_culling', 'Frustum culling'],
  ['show_frustum', 'Frustum lines'],
  ['show_culled_boxes', 'Culled boxes'],
  ['player_cam_view', 'Player cam view'],
];

export interface DebugOverlayApi {
  init(seriesNames: string[]): void;
  setVisible(on: boolean): void;
  setPaused(on: boolean): void;
  setToggles(state: Record<string, boolean>): void;
  setInfo(text: string): void;
  setGraph(data: Float64Array, avgs: Float64Array): void;
}

interface State {
  names: string[];
  visible: boolean;
  paused: boolean;
  toggles: Record<string, boolean>;
  info: string;
  lastGraph: { data: Float64Array; avgs: Float64Array } | null;
}

type Impl = DebugOverlayApi & { destroy(): void };

interface Facade extends DebugOverlayApi {
  impl: Impl | null;
  state: State;
  setToggle: (name: string, on: boolean) => void;
  opts: { docked: boolean };
}

function build(
  setToggle: (name: string, on: boolean) => void,
  opts: { docked: boolean },
  state: State,
): Impl {
  const root = document.createElement('div');
  root.id = 'rwg-debug';
  root.className = opts.docked ? 'docked' : 'overlay';
  if (opts.docked) document.body.classList.add('debug-docked');
  root.style.visibility = 'hidden';

  const info = document.createElement('pre');
  info.className = 'info';

  const pause = document.createElement('div');
  pause.className = 'pause';
  pause.hidden = true;
  const title = document.createElement('div');
  title.className = 'title';
  title.textContent = 'Game is paused';
  pause.append(title);

  const boxes = new Map<string, HTMLInputElement>();
  for (const [name, label] of TOGGLES) {
    const l = document.createElement('label');
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    // No optimistic update: revert, wait for Rust's setToggles echo.
    cb.addEventListener('change', () => {
      const want = cb.checked;
      cb.checked = !want;
      setToggle(name, want);
    });
    l.append(cb, ' ' + label);
    pause.append(l);
    boxes.set(name, cb);
  }

  const row = document.createElement('div');
  row.className = 'row';
  const copy = document.createElement('button');
  copy.textContent = 'Copy';
  copy.addEventListener('click', () => {
    navigator.clipboard?.writeText(info.textContent ?? '').catch(() => { });
  });
  let graphOn = false;
  const graphBtn = document.createElement('button');
  graphBtn.textContent = 'Show graph';
  graphBtn.addEventListener('click', () => setToggle('graph', !graphOn));
  row.append(copy, graphBtn);
  pause.append(row);

  const wrap = document.createElement('div');
  wrap.className = 'graph-wrap';
  wrap.hidden = true;
  const graph = document.createElement('canvas');
  graph.className = 'graph';
  const legend = document.createElement('div');
  legend.className = 'legend';
  wrap.append(legend, graph);
  const ctx = graph.getContext('2d');

  root.append(info, pause, wrap);
  document.body.append(root);

  let names: string[] = [];
  let vals: HTMLElement[] = [];
  let lastVals: string[] = [];
  let last: State['lastGraph'] = null;
  let base = new Float64Array(0);

  const draw = () => {
    if (!ctx || !graphOn || !last) return;
    const { data, avgs } = last;
    const dpr = window.devicePixelRatio || 1;
    const n = names.length || avgs.length;
    const len = n ? Math.floor(data.length / n) : 0;
    const w = graph.width;
    const h = graph.height;
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = 'rgba(55,55,55,0.8)';
    ctx.fillRect(0, 0, w, h);
    if (len < 2) return;
    if (base.length !== len) base = new Float64Array(len);
    base.fill(0);
    let max = 0;
    for (let i = 0; i < len; i++) {
      let t = 0;
      for (let s = 0; s < n; s++) t += data[s * len + i];
      if (t > max) max = t;
    }
    if (max <= 0) return;
    const X = (i: number) => (i / (len - 1)) * w;
    const Y = (v: number) => h - (v / max) * (h - 2) - 1;
    ctx.lineWidth = dpr;
    for (let s = 0; s < n; s++) {
      const color = COLORS[s % COLORS.length];
      ctx.beginPath();
      for (let i = 0; i < len; i++) {
        const y = Y(base[i] + data[s * len + i]);
        i ? ctx.lineTo(X(i), y) : ctx.moveTo(X(i), y);
      }
      for (let i = len - 1; i >= 0; i--) ctx.lineTo(X(i), Y(base[i]));
      ctx.closePath();
      ctx.globalAlpha = 0.85;
      ctx.fillStyle = color;
      ctx.fill();
      ctx.globalAlpha = 1;
      // thin stroke on top edge for band separation
      ctx.strokeStyle = color;
      ctx.beginPath();
      for (let i = 0; i < len; i++) {
        const y = Y(base[i] + data[s * len + i]);
        i ? ctx.lineTo(X(i), y) : ctx.moveTo(X(i), y);
      }
      ctx.stroke();
      for (let i = 0; i < len; i++) base[i] += data[s * len + i];
    }
    // guide lines + labels
    ctx.font = `${Math.round(9 * dpr)}px monospace`;
    ctx.fillStyle = 'rgba(255,255,255,0.7)';
    ctx.strokeStyle = 'rgba(255,255,255,0.35)';
    ctx.setLineDash([3 * dpr, 3 * dpr]);
    ctx.textAlign = 'right';
    ctx.textBaseline = 'bottom';
    for (const ms of [16.6, 33.3]) {
      if (ms >= max) continue;
      const y = Y(ms);
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(w, y);
      ctx.stroke();
      ctx.fillText(`${ms}ms`, w - 2 * dpr, y - dpr);
    }
    ctx.setLineDash([]);
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    ctx.fillText(`${max.toFixed(1)} ms`, 2 * dpr, 2 * dpr);
  };

  // Size comes from CSS; keep the backing store matched. Assigning width/height clears
  // the canvas, so only touch them on change, then redraw.
  const ro = new ResizeObserver(() => {
    const dpr = window.devicePixelRatio || 1;
    const w = Math.round(graph.clientWidth * dpr);
    const h = Math.round(graph.clientHeight * dpr);
    if (w <= 0 || h <= 0) return;
    if (w === graph.width && h === graph.height) return;
    graph.width = w;
    graph.height = h;
    draw();
  });
  ro.observe(graph);

  const impl: Impl = {
    init(seriesNames) {
      names = seriesNames;
      vals = [];
      lastVals = [];
      legend.replaceChildren();
      names.forEach((name, s) => {
        const item = document.createElement('span');
        item.className = 'item';
        item.style.setProperty('--c', COLORS[s % COLORS.length]);
        const sw = document.createElement('i');
        sw.className = 'sw';
        const nm = document.createElement('b');
        nm.className = 'name';
        nm.textContent = name;
        const val = document.createElement('span');
        val.className = 'val';
        item.append(sw, nm, ' ', val, ' ms');
        legend.append(item);
        vals.push(val);
      });
    },
    setVisible(on) {
      root.style.visibility = on ? 'visible' : 'hidden';
    },
    setPaused(on) {
      pause.hidden = !on;
    },
    setToggles(t) {
      for (const [k, v] of Object.entries(t)) {
        if (k === 'graph') {
          graphOn = v;
          wrap.hidden = !v;
          graphBtn.textContent = v ? 'Hide graph' : 'Show graph';
        } else {
          const cb = boxes.get(k);
          if (cb) cb.checked = v;
        }
      }
    },
    setInfo(text) {
      info.textContent = text;
    },
    setGraph(data, avgs) {
      if (!graphOn) return;
      last = { data, avgs };
      for (let s = 0; s < vals.length; s++) {
        const t = avgs[s]?.toFixed(2) ?? '';
        if (t !== lastVals[s]) {
          vals[s].textContent = t;
          lastVals[s] = t;
        }
      }
      draw();
    },
    destroy() {
      ro.disconnect();
      root.remove();
      document.body.classList.remove('debug-docked');
    },
  };

  // Replay remembered state into the fresh DOM.
  impl.init(state.names);
  impl.setVisible(state.visible);
  impl.setPaused(state.paused);
  impl.setToggles(state.toggles);
  impl.setInfo(state.info);
  if (state.lastGraph) impl.setGraph(state.lastGraph.data, state.lastGraph.avgs);
  return impl;
}

export function installDebugOverlay(
  setToggle: (name: string, on: boolean) => void,
  opts: { docked: boolean },
): DebugOverlayApi {
  const state: State = {
    names: [],
    visible: false,
    paused: false,
    toggles: {},
    info: '',
    lastGraph: null,
  };
  const facade: Facade = {
    impl: null,
    state,
    setToggle,
    opts,
    init(names) {
      state.names = names;
      facade.impl?.init(names);
    },
    setVisible(on) {
      state.visible = on;
      facade.impl?.setVisible(on);
    },
    setPaused(on) {
      state.paused = on;
      facade.impl?.setPaused(on);
    },
    setToggles(t) {
      Object.assign(state.toggles, t);
      facade.impl?.setToggles(t);
    },
    setInfo(text) {
      state.info = text;
      facade.impl?.setInfo(text);
    },
    setGraph(data, avgs) {
      // Rust may reuse/overwrite buffers; copy so replay stays valid.
      state.lastGraph = { data, avgs };
      facade.impl?.setGraph(data, avgs);
    },
  };
  facade.impl = build(setToggle, opts, state);
  (window as any).__rwgDebug = facade;
  return facade;
}

if (import.meta.hot) {
  import.meta.hot.accept();
  import.meta.hot.dispose(() => {
    const f = (window as any).__rwgDebug as Facade | undefined;
    f?.impl?.destroy();
    if (f) f.impl = null;
  });
  const f = (window as any).__rwgDebug as Facade | undefined;
  if (f && f.impl === null) f.impl = build(f.setToggle, f.opts, f.state);
}
