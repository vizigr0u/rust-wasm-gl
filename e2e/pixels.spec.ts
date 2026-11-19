import { test, expect } from '@playwright/test';
import { startScene, settle, setCamera } from './helpers';

// Keep in sync with voxel-core camera::tests::single_block_cameras
// Player spawns at y=16 (chunk 0,1,0); the block is in chunk (0,0,0), i.e. 16 below, relative to the player chunk.
const SINGLE_BLOCK_CAMERAS: [string, number[]][] = [
  ['pos_x', [3, 0, 0]],
  ['neg_x', [-3, 0, 0]],
  ['pos_z', [0, 0, 3]],
  ['neg_z', [0, 0, -3]],
  ['top', [0.3, 3, 0.3]],
  ['bottom', [0.3, -3, 0.3]],
  ['three_quarter', [2, 1.5, 2]],
];

// Page screenshot clipped to the canvas: skips the element-stability wait,
// which never resolves in the default_spawn scene.
const SHOT = { clip: { x: 0, y: 0, width: 800, height: 600 } };

test('default_spawn', async ({ page }) => {
  await startScene(page, 'default_spawn');
  await settle(page);
  await setCamera(page, [0, 1, 0], [-10, 5, 0]);
  await expect(page).toHaveScreenshot('default_spawn.png', SHOT);
});

for (const [name, offset] of SINGLE_BLOCK_CAMERAS) {
  test(`single_block ${name}`, async ({ page }) => {
    await startScene(page, 'single_block');
    await settle(page);
    await setCamera(page, [0.5, -15.5, 0.5], offset);
    await expect(page).toHaveScreenshot(`single_block_${name}.png`, SHOT);
  });
}
