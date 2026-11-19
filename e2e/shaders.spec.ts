import { test, expect } from '@playwright/test';
import { startScene } from './helpers';

test('all shaders compile', async ({ page }) => {
  await startScene(page, 'default_spawn');
  const errors = await page.evaluate(() => (window as any).__rwg.test_compile_all_shaders());
  expect(errors).toEqual([]);
});
