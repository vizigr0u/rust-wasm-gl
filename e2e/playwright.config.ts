import { defineConfig } from '@playwright/test';

const tier = process.env.PIXEL_TIER ?? 'strict';
const PORT = 4173;

const webgpuArgs = [
  '--enable-unsafe-webgpu',
  '--enable-features=Vulkan',
  '--use-vulkan=swiftshader',
  '--use-angle=swiftshader',
];

export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.ts',
  snapshotPathTemplate: `{testDir}/__screenshots__/${process.env.GOLDEN_DIR ?? 'webgpu'}/{arg}{ext}`,
  outputDir: '../test-results',
  retries: 2,
  fullyParallel: true,
  workers: process.env.CI ? 2 : 4,
  timeout: 120_000,
  reporter: [['list']],
  expect: {
    timeout: 30_000,
    toHaveScreenshot:
      tier === 'loose'
        ? { maxDiffPixelRatio: 0.02 }
        : { threshold: 0.1, maxDiffPixelRatio: 0.001 },
  },
  use: {
    baseURL: `http://localhost:${PORT}`,
    viewport: { width: 800, height: 600 },
    deviceScaleFactor: 1,
    launchOptions: { args: webgpuArgs },
  },
  webServer: {
    // build wasm (test-hooks) + bundle, then serve
    command: `pnpm wasm:test && pnpm exec vite build && pnpm exec vite preview --port ${PORT} --strictPort`,
    cwd: '..',
    url: `http://localhost:${PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 600_000,
  },
});
