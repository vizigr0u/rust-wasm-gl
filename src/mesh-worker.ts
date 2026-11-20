// Meshing worker: runs the same wasm module as the page, only its `mesh_job` export.
import init, { mesh_job } from '../backend/pkg';

const ready = init();
let trace = false;

self.onmessage = async (e: MessageEvent<{ id: number; input: Uint8Array } | { trace: boolean }>) => {
  if ('trace' in e.data) {
    trace = e.data.trace;
    return;
  }
  await ready;
  const { id, input } = e.data;
  const t0 = trace ? performance.now() : 0;
  const output = mesh_job(input);
  // Extended console.timeStamp: custom track in the DevTools Performance panel.
  if (trace) (console as any).timeStamp('mesh', t0, performance.now(), 'Mesher', 'Engine', 'tertiary');
  (self as unknown as Worker).postMessage({ id, output }, [output.buffer]);
};
