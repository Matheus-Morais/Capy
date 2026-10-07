import test from 'node:test';
import assert from 'node:assert/strict';
import { PetBehavior, hoverGaze } from '../src/pet-behavior.ts';

const session = (id, state, extra = {}) => ({ id, state, hidden:false, ...extra });
const data = (sessions = [], extra = {}) => ({ scenario:'real', sessions, interventions:[], reduceMotion:false, ...extra });

test('greeting happens once and ends without replay on snapshots', () => {
  const pet = new PetBehavior(0);
  assert.equal(pet.update(data(), 0).gesture, 'hello');
  assert.equal(pet.update(data(), 1799).gesture, 'hello');
  assert.equal(pet.update(data(), 1800).gesture, '');
  assert.equal(pet.update(data(), 2000).gesture, '');
});
test('sleep begins only after three minutes and interaction wakes', () => {
  const pet = new PetBehavior(100);
  assert.equal(pet.update(data(), 180099).state, 'idle');
  assert.equal(pet.update(data(), 180100).state, 'sleeping');
  pet.interact(180101);
  assert.equal(pet.update(data(), 180101).state, 'idle');
  assert.equal(pet.update(data([session('a', 'idle')]), 360101).state, 'sleeping');
});
test('sleep cannot occur during work or a pending request even when seen', () => {
  for (const state of ['working', 'waiting']) {
    const pet = new PetBehavior(0);
    pet.update(data([session('a', state)]), 1);
    pet.acknowledge();
    assert.equal(pet.update(data([session('a', state)]), 600000).state, state);
    assert.equal(pet.update(data(), 779999).state, 'idle');
    assert.equal(pet.update(data(), 780000).state, 'sleeping');
  }
});
test('attention waves once per new request and repeats only while unseen', () => {
  const pet = new PetBehavior(0);
  const waiting = data([session('a', 'waiting')]);
  const first = pet.update(waiting, 0);
  assert.equal(first.gesture, 'wave');
  assert.equal(first.waiting, 1);
  assert.equal(pet.update(waiting, 1599).gestureId, first.gestureId);
  assert.equal(pet.update(waiting, 1600).gesture, '');
  assert.equal(pet.update(waiting, 29999).gesture, '');
  assert.equal(pet.update(waiting, 30000).gesture, 'wave');
  pet.acknowledge();
  assert.equal(pet.update(waiting, 60000).gesture, '');
  pet.update(data(), 60001);
  assert.equal(pet.update(waiting, 60002).gesture, 'wave');
});
test('attention recognizes a new exact request in the same waiting session', () => {
  const pet = new PetBehavior(0);
  const withRequest = nonce => data([session('a', 'waiting')], {interventions:[{sessionId:'a', status:'pending', generation:'g', nonce}]});
  pet.update(withRequest('one'), 0);
  pet.acknowledge();
  assert.equal(pet.update(withRequest('one'), 3000).gesture, '');
  assert.equal(pet.update(withRequest('two'), 3001).gesture, 'wave');
  assert.equal(pet.update(data([session('a', 'waiting', {hidden:true})]), 3002).waiting, 0);
});
test('attention waves for a chat review by nonce and clears after cancellation or approval',()=>{
  const pet=new PetBehavior(0);
  const review=nonce=>data([session('chat:a','idle')],{chatTransfers:[{nonce,sourceId:'a'}]});
  assert.equal(pet.update(review('one'),2000).gesture,'wave');assert.equal(pet.update(review('one'),2001).waiting,1);
  pet.acknowledge();assert.equal(pet.update(review('one'),5000).gesture,'');
  assert.equal(pet.update(review('two'),5001).gesture,'wave');assert.equal(pet.update(data(),5002).waiting,0);
});
test('completion requires explicit real event and never infers from stop or disappearance', () => {
  const pet = new PetBehavior(0);
  for (const state of ['working', 'idle', 'unknown', 'done']) {
    assert.notEqual(pet.update(data([session('a', state)]), 2000).gesture, 'celebrate');
  }
  assert.notEqual(pet.update(data(), 2001).gesture, 'celebrate');
  const completed = data([session('a', 'idle', {completion:'turn-1'})]);
  assert.equal(pet.update(completed, 2002).gesture, 'celebrate');
  assert.equal(pet.update(completed, 4202).gesture, '');
  assert.equal(pet.update(completed, 4203).state, 'idle');
  assert.equal(pet.update(data([session('a','idle',{completion:'turn-2'})]), 5000).gesture, 'celebrate');
});
test('completion in demonstration celebrates once per done transition', () => {
  const pet = new PetBehavior(0);
  const completed = data([session('a', 'done')], {scenario:'done'});
  assert.equal(pet.update(completed, 2000).state, 'done');
  assert.equal(pet.update(completed, 4200).state, 'idle');
  pet.update(data([session('a','working')], {scenario:'working'}), 4201);
  assert.equal(pet.update(completed, 4202).gesture, 'celebrate');
});
test('completion celebrates one session while preserving other work and attention priority', () => {
  const pet = new PetBehavior(0);
  const completed = data([session('finished','idle',{completion:'turn-1'}),session('other','working')]);
  const frame = pet.update(completed,2000);
  assert.equal(frame.gesture,'celebrate');assert.equal(frame.state,'working');
  assert.equal(pet.update(completed,4200).gesture,'');assert.equal(pet.update(completed,4201).state,'working');
  const waiting = data([session('finished','idle',{completion:'turn-2'}),session('other','waiting')]);
  assert.equal(pet.update(waiting,5000).gesture,'wave');assert.equal(pet.update(waiting,5001).state,'waiting');
});
test('click gesture ends and preserves base work state', () => {
  const pet = new PetBehavior(0);
  pet.interact(2000, true);
  assert.equal(pet.update(data([session('a','working')]), 2000).gesture, 'click');
  assert.equal(pet.update(data([session('a','working')]), 2650).state, 'working');
  assert.equal(pet.update(data(), 2650).gesture, '');
});
test('hover gaze remains bounded to mascot coordinates', () => {
  assert.deepEqual(hoverGaze(90,70,180,140), [0,0]);
  assert.deepEqual(hoverGaze(10000,-10000,180,140), [2.5,-1.5]);
  assert.deepEqual(hoverGaze(0,0,0,0), [0,0]);
});
