import { Page, expect } from '@playwright/test';

export async function startScene(page: Page, scene: string) {
  page.on('pageerror', (e) => console.log('pageerror:', e.message));
  await page.addInitScript(() => ((window as any).__RWG_TEST__ = true));
  await page.goto('/');
  await page.waitForFunction(() => (window as any).__rwg?.test_start);
  await page.evaluate((s) => (window as any).__rwg.test_start(s), scene);
}

/** Update-only frames inside wasm until settled, then one rendered frame. */
export async function settle(page: Page) {
  const frames = await page.evaluate(() => (window as any).__rwg.test_settle(5000, 16));
  expect(frames, 'world did not settle').toBeLessThan(5000);
}

export async function setCamera(page: Page, target: number[], offset: number[]) {
  await page.evaluate(
    ([t, o]) => {
      const w = (window as any).__rwg;
      w.test_set_camera(t, o);
      w.test_step(2, 16);
    },
    [target, offset],
  );
}
