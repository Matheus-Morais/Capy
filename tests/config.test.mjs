import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

test('companion window is transparent frameless topmost with a compact footprint', () => {
  const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
  const pet = config.app.windows.find(w => w.label === 'pet');
  assert.equal(pet.transparent, true);
  assert.equal(pet.decorations, false);
  assert.equal(pet.alwaysOnTop, true);
  assert.equal(pet.skipTaskbar, true);
  assert.equal(pet.width, 200);
  assert.equal(pet.height, 180);
  assert.deepEqual(config.app.windows.filter(w => w.visible).map(w => w.label), []);
});

test('frontend capabilities do not expose shell or unrestricted filesystem', () => {
  const capabilities = JSON.parse(readFileSync('src-tauri/capabilities/main.json', 'utf8'));
  assert.deepEqual(capabilities.permissions, ['core:event:allow-listen', 'core:event:allow-unlisten', 'core:window:allow-start-dragging']);
});
