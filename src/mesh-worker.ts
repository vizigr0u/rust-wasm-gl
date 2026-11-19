// Meshing worker: runs the same wasm module as the page, only its `mesh_job` export.
import init, { mesh_job } from '../backend/pkg';

const ready = init();

self.onmessage = async (e: MessageEvent<{ id: number; input: Uint8Array }>) => {
  await ready;
  const { id, input } = e.data;
  const output = mesh_job(input);
  (self as unknown as Worker).postMessage({ id, output }, [output.buffer]);
};
