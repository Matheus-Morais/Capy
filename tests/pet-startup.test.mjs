import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {stripTypeScriptTypes} from 'node:module';
import {PetBehavior, hoverGaze} from '../src/pet-behavior.ts';

// Execute the entry point with a minimal window, retaining lexical initialization semantics.
function boot() {
  const source = readFileSync(new URL('../src/pet.ts', import.meta.url), 'utf8')
    .replace(/^import .*;\r?$/gm, '');
  const script = stripTypeScriptTypes(source);
  const elements = new Map();
  const errors = [], commands = [], sounds = [];
  function element() {
    const classes = new Set(), listeners = new Map(), attrs = new Map();
    return {listeners, hidden:false, textContent:'', title:'',
      classList:{add:(...values)=>values.forEach(v=>classes.add(v)), remove:(...values)=>values.forEach(v=>classes.delete(v)),
        contains:v=>classes.has(v), toggle:(v,on)=>on ? classes.add(v) : classes.delete(v)},
      style:{removeProperty(){}, setProperty(){}}, querySelector:()=>null,
      getBoundingClientRect:()=>({left:0,top:0,width:200,height:180}),
      addEventListener:(name,callback)=>listeners.set(name,callback),
      setAttribute:(name,value)=>attrs.set(name,value), getAttribute:name=>attrs.get(name),
    };
  }
  for (const id of ['petArtwork','petToggle','petBadge','petStatus']) elements.set(id,element());
  Object.defineProperty(elements.get('petArtwork'),'innerHTML',{set(){elements.set('pet',element());}});
  class Clock extends Date { static now() { return 0; } }
  const data = {scenario:'real',sessions:[],interventions:[],reduceMotion:false,preferences:{sounds:true}};
  const context = vm.createContext({PetBehavior,hoverGaze,Date:Clock,Math,
    artwork:'v5',artworkV1:'v1',artworkV2:'v2',artworkV3:'v3',artworkV4:'v4',
    localStorage:{getItem:()=>null,setItem(){}},
    document:{getElementById:id=>elements.get(id),body:element(),hidden:false,addEventListener(){}},
    window:{addEventListener(){},setTimeout:()=>0},setTimeout:()=>0,clearTimeout(){},setInterval:()=>0,
    native:false,subscribe:async()=>{},subscribeVisibility:async()=>{},snapshot:async()=>data,
    desktopCommand:async name=>commands.push(name),showError:error=>errors.push(error),
    playPetSound:sound=>sounds.push(sound),
  });
  vm.runInContext(script,context,{filename:'pet.ts'});
  return {elements,errors,commands,sounds};
}

test('pet entry initializes artwork listeners and reports ready without runtime errors',async()=>{
  const app = boot();
  await new Promise(resolve=>setImmediate(resolve));
  assert.deepEqual(app.errors,[]);
  assert.ok(app.commands.includes('ui_ready'));
  assert.ok(app.elements.get('pet').classList.contains('s-idle'));
  assert.ok(app.elements.get('pet').classList.contains('g-hello'));
  assert.ok(app.elements.get('petToggle').listeners.has('click'));
});

test('skin replacement preserves the active greeting without another sound',async()=>{
  const app = boot();
  await new Promise(resolve=>setImmediate(resolve));
  const original = app.elements.get('pet');
  app.elements.get('petToggle').listeners.get('keydown')({key:'v',preventDefault(){}});
  assert.notEqual(app.elements.get('pet'),original);
  assert.ok(app.elements.get('pet').classList.contains('s-idle'));
  assert.ok(app.elements.get('pet').classList.contains('g-hello'));
  assert.deepEqual(app.sounds,['hello']);
});
