// Profiling scenarios. Pure data: the runner serialises them into the page.
//
// pose:  { fixed: [target, offset] } | { orbit: { radius, height } } | null (camera follows the player)
//        target/offset are relative to the player's chunk (see test_set_camera).
// move:  blocks per frame the player is teleported by (drives chunk streaming), or null.
// toggles: switches applied after the world settled (see Game::set_toggle).
//
// Add a scenario here and it runs, no other change needed. Keep the names stable: baselines
// are matched by name.

/** @typedef {{name:string, desc:string, frames:number, pose:any, move:number[]|null, toggles:Record<string,boolean>}} Scenario */

const NEAR = [[0, 1, 0], [-10, 5, 0]];
const WIDE = [[0, 1, 0], [-80, 60, 0]];

/** @type {Scenario[]} */
export const SCENARIOS = [
  {
    name: 'near_static',
    desc: 'default spawn view, frustum culling on, world settled: steady-state frame cost',
    frames: 240,
    pose: { fixed: NEAR },
    move: null,
    toggles: {},
  },
  {
    name: 'near_static_nocull',
    desc: 'same view, culling off: cost of drawing everything the arena holds',
    frames: 240,
    pose: { fixed: NEAR },
    move: null,
    toggles: { frustum_culling: false },
  },
  {
    name: 'wide_static_nocull',
    desc: 'pulled-back view, culling off: worst-case geometry / draw-call load',
    frames: 240,
    pose: { fixed: WIDE },
    move: null,
    toggles: { frustum_culling: false },
  },
  {
    name: 'orbit_cull',
    desc: 'camera circles the player, culling on: varying visible set, cull + draw-list cost',
    frames: 360,
    pose: { orbit: { radius: 40, height: 20 } },
    move: null,
    toggles: {},
  },
  {
    name: 'orbit_nocull',
    desc: 'camera circles the player, culling off: same path as orbit_cull for A/B',
    frames: 360,
    pose: { orbit: { radius: 40, height: 20 } },
    move: null,
    toggles: { frustum_culling: false },
  },
  {
    name: 'stream_walk',
    desc: 'player flies 1 block/frame: streaming, inline meshing, arena alloc + uploads',
    frames: 400,
    pose: null,
    move: [1, 0, 0],
    toggles: {},
  },
  {
    name: 'ui_near',
    desc: 'near view with egui on: UI build + egui pass cost',
    frames: 240,
    pose: { fixed: NEAR },
    move: null,
    toggles: { egui: true },
  },
];
