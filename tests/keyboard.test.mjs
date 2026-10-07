import test from 'node:test';
import assert from 'node:assert/strict';
import {shouldCloseOnEscape} from '../src/keyboard.ts';

test('Escape stays with the terminal, edited form or dialog, and only closes an unhandled panel',()=>{
  const base={defaultPrevented:false,composing:false,terminal:false,editing:false,dialog:false};
  assert.equal(shouldCloseOnEscape('Escape',base),true);
  assert.equal(shouldCloseOnEscape('Enter',base),false);
  for(const key of Object.keys(base))assert.equal(shouldCloseOnEscape('Escape',{...base,[key]:true}),false,key);
});
