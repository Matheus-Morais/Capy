import test from 'node:test';
import assert from 'node:assert/strict';
import { PetLifeEngine, FIDGET_CATALOG } from '../src/pet-life.ts';

test('FIDGET_CATALOG has valid durations, weights, and non-empty classes', () => {
  assert.ok(FIDGET_CATALOG.length >= 8);
  for (const item of FIDGET_CATALOG) {
    assert.ok(item.durationMs > 0);
    assert.ok(item.weight > 0);
    assert.ok(item.cooldownMs > 0);
    assert.ok(item.className.length > 0);
  }
});

test('PetLifeEngine triggers fidgets when canFidget is true and respects cancelActiveGesture', () => {
  const gestures = [];
  const removed = [];
  const gazes = [];
  let allowed = true;

  const engine = new PetLifeEngine({
    applyGesture(cls, duration, sound) { gestures.push({ cls, duration, sound }); },
    removeGesture(cls) { removed.push(cls); },
    setGaze(x, y) { gazes.push([x, y]); },
    canFidget() { return allowed; },
  });

  // Manually invoke internal trigger logic
  // @ts-ignore
  engine.tryTriggerFidget();
  assert.equal(gestures.length, 1);
  assert.ok(gestures[0].cls.length > 0);

  // Cancelling active gesture
  engine.cancelActiveGesture();
  assert.equal(removed.length, 1);
  assert.equal(removed[0], gestures[0].cls);

  // When canFidget is false, no new gesture is triggered
  allowed = false;
  // @ts-ignore
  engine.tryTriggerFidget();
  assert.equal(gestures.length, 1);

  engine.stop();
});

test('PetLifeEngine handles global gaze and gaze drift', () => {
  const gazes = [];
  const engine = new PetLifeEngine({
    applyGesture() {},
    removeGesture() {},
    setGaze(x, y) { gazes.push([x, y]); },
    canFidget() { return true; },
  });

  engine.setGlobalGaze(1.5, -0.8);
  assert.deepEqual(gazes[gazes.length - 1], [1.5, -0.8]);

  // While global gaze is active, tryGazeDrift should NOT overwrite it
  const countBefore = gazes.length;
  // @ts-ignore
  engine.tryGazeDrift();
  assert.equal(gazes.length, countBefore);

  // After clearing global gaze, drift is allowed
  engine.clearGlobalGaze();
  // @ts-ignore
  engine.tryGazeDrift();
  assert.ok(gazes.length > countBefore);

  engine.stop();
});

test('PetLifeEngine drag kinematics calculates tilt and squash-and-stretch within clamp bounds', () => {
  const engine = new PetLifeEngine({
    applyGesture() {},
    removeGesture() {},
    setGaze() {},
    canFidget() { return true; },
  });

  engine.onDragStart(100, 100);
  // Fast horizontal drag to the right
  const rightDrag = engine.onDragMove(130, 102);
  assert.ok(rightDrag.tilt > 0);
  assert.ok(rightDrag.tilt <= 12);
  assert.ok(rightDrag.scaleX > 0.8 && rightDrag.scaleX < 1.2);

  // Fast downward drag
  const downDrag = engine.onDragMove(130, 150);
  assert.ok(downDrag.scaleY >= 1.0);
  assert.ok(downDrag.scaleY <= 1.15);
  assert.ok(downDrag.scaleX <= 1.0);

  engine.stop();
});
