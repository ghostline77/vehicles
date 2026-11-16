'use strict';
(()=>{
const data=globalThis.GHOSTLINE_VEHICLES,$=id=>document.getElementById(id),records=new Map(data.records.map(v=>[v.id,v])),selections=new Map(data.selections.map(s=>[s.id,s]));
const node=(tag,text,cls)=>{const n=document.createElement(tag);if(text!=null)n.textContent=String(text);if(cls)n.className=cls;return n;};
const fmt=(v,unit='')=>typeof v==='number'?new Intl.NumberFormat(undefined,{maximumFractionDigits:1}).format(v)+unit:'Unavailable';
const spec=v=>v.performance?.specifications||{},top=v=>v.performance?.calculated?.top_gear||{};
let page=0,detailGeneration=0;const pageSize=36;
let compared=new Map();try{compared=new Map(JSON.parse(localStorage.getItem('ghostline-vehicles-compare')||'[]').filter(entry=>Array.isArray(entry)&&records.has(entry[1]?.record)));}catch{}
const appearanceChoices=new Map(),compareKey=(v,s)=>JSON.stringify([v.id,s?.id??null]);
const save=()=>{try{localStorage.setItem('ghostline-vehicles-compare',JSON.stringify([...compared]));}catch{}};
const mediaUrl=m=>data.media_base?data.media_base.replace(/\/$/,'')+'/'+m.path:m.path;
function copy(value){const text=JSON.stringify(value,null,2);if(navigator.clipboard?.writeText)navigator.clipboard.writeText(text).catch(()=>fallback(text));else fallback(text);}
function fallback(text){const input=node('textarea');input.value=text;document.body.append(input);input.select();try{document.execCommand('copy');}finally{input.remove();}$('status').textContent='Identity selected for copying.';}
function button(text,fn){const b=node('button',text);b.type='button';b.onclick=fn;return b;}
function selectionList(v){return v.selections.map(id=>selections.get(id)).filter(Boolean);}
function visual(s){
 const box=node('div',null,'visual'),views=node('div',null,'views');
 if(!s?.media.length){box.append(node('div',s?({blocked:'Bindings need resolution',failed:'Render failed',not_applicable:'No visible preview geometry',pending:'Preview pending'}[s.preview_state]||'Preview unavailable'):'No visual selection','placeholder'));return{box,views};}
 const img=node('img');img.loading='lazy';img.alt=`${s.appearance||'Native vehicle'} — ${s.media[0].view}`;img.src=mediaUrl(s.media[0]);
 img.onerror=()=>{box.replaceChildren(node('p','Image unavailable','placeholder'));};
 img.onclick=()=>{const dialog=$('image-viewer');dialog.querySelector('img').src=img.src;dialog.querySelector('img').alt=img.alt;dialog.querySelector('p').textContent=s.id;dialog.showModal();};box.append(img);
 for(const m of s.media){const b=button(m.view,()=>{img.src=mediaUrl(m);img.alt=`${s.appearance||'Native vehicle'} — ${m.view}`;for(const other of views.children)other.setAttribute('aria-pressed',String(other===b));});b.setAttribute('aria-pressed',String(m===s.media[0]));views.append(b);}
 return{box,views};
}
function specs(v){const dl=node('dl',null,'specs');for(const[k,value]of[
 ['Horsepower',fmt(spec(v).horsepower,' hp')],['Physics mass',fmt(spec(v).physics_mass_kg,' kg')],['Calculated HUD ceiling',fmt(top(v).hud_mph,' mph')],['World-space gearing ceiling',fmt(top(v).world_kmh,' km/h')]]){const pair=node('div');pair.append(node('dt',k),node('dd',value));dl.append(pair);}return dl;}
function compare(){
 $('comparison').hidden=!compared.size;const table=$('compare-table');table.replaceChildren();const chosen=[...compared.values()].map(entry=>({v:records.get(entry.record),s:selections.get(entry.selection)})).filter(entry=>entry.v);
 const images=node('tr');images.append(node('th','Appearance preview'));for(const entry of chosen){const cell=node('td'),image=visual(entry.s);image.box.style.height='165px';cell.append(image.box);images.append(cell);}table.append(images);
 const appearances=node('tr');appearances.append(node('th','Exact appearance'));for(const entry of chosen)appearances.append(node('td',entry.s?.appearance||'Unspecified'));table.append(appearances);
 const rows=[['Vehicle',v=>`${v.label} (${v.native_name||v.id})`],['Record appearance request',v=>v.requested_appearance||'Unspecified'],['Horsepower',v=>fmt(spec(v).horsepower,' hp')],['UI mass',v=>fmt(spec(v).ui_mass_kg,' kg')],['Physics mass',v=>fmt(spec(v).physics_mass_kg,' kg')],['Calculated HUD ceiling',v=>`${fmt(top(v).hud_mph,' mph')} / ${fmt(top(v).hud_kmh,' km/h')}`],['World-space gearing ceiling',v=>`${fmt(top(v).world_mph,' mph')} / ${fmt(top(v).world_kmh,' km/h')}`],['Max RPM',v=>fmt(spec(v).max_rpm)],['Engine torque (native parameter)',v=>fmt(spec(v).engine_torque)],['Evidence',()=> 'Authored specifications and calculated gearing ceilings; no gameplay measurements']];
 for(const[label,get]of rows){const tr=node('tr');tr.append(node('th',label));for(const entry of chosen)tr.append(node('td',get(entry.v)));table.append(tr);}
}
function toggleCompare(v,s){const key=compareKey(v,s);if(compared.has(key))compared.delete(key);else if(compared.size<4)compared.set(key,{record:v.id,selection:s?.id??null});else{$('status').textContent='Compare up to four record/appearance selections.';return;}save();compare();render();}
function card(v){
 const article=node('article',null,'card'),body=node('div',null,'body');const list=selectionList(v);let current=list.find(s=>s.id===appearanceChoices.get(v.id))||list.find(s=>s.media.length)||list[0];const initial=visual(current);article.append(initial.box);body.append(node('p',[v.manufacturer,v.vehicle_type].filter(Boolean).join(' / '),'eyebrow'));
 const a=node('a',v.label);a.href='#/vehicle/'+encodeURIComponent(v.id);const h=node('h2');h.append(a);body.append(h,node('p',v.native_name||v.id,'identity'));
 if(list.length){const select=node('select');select.setAttribute('aria-label',`Appearance for ${v.label}`);for(const s of list){const option=node('option',s.appearance||'Unspecified native appearance');option.value=s.id;select.append(option);}select.value=current.id;body.append(select);
  select.onchange=()=>{current=selections.get(select.value);appearanceChoices.set(v.id,current.id);const next=visual(current);article.replaceChild(next.box,article.firstChild);viewHost.replaceChildren(...next.views.children);compareButton.textContent=compared.has(compareKey(v,current))?'Remove comparison':'Compare';};}
 const viewHost=node('div',null,'views');viewHost.append(...initial.views.children);body.append(viewHost,specs(v));const actions=node('div',null,'actions');
 if(!v.requested_appearance||v.requested_appearance==='random')body.append(node('p',v.requested_appearance==='random'?'The record selects its appearance at runtime; this preview is an explicit authored selection.':'No appearance specified by the record; choose an authored entity appearance.','notice'));
 const compareButton=button(compared.has(compareKey(v,current))?'Remove comparison':'Compare',()=>toggleCompare(v,current));
 actions.append(compareButton,button('Copy identity',()=>copy({record_id:v.native_name||v.id,native_record_id:v.id,entity:v.entity,record_requested_appearance:v.requested_appearance,appearance:current?.appearance??null,source:data.sources})));
 if(current)actions.append(button('Appearance details',()=>{location.hash='#/selection/'+encodeURIComponent(current.id);}));
 body.append(actions);if(!list.length||v.diagnostics.length)body.append(node('p',v.diagnostics[0]||'Metadata-only record','notice'));article.append(body);return article;
}
function render(){
 const q=$('search').value.trim().toLowerCase(),make=$('manufacturer').value,type=$('type').value;
 let rows=data.records.filter(v=>(!make||v.manufacturer===make)&&(!type||v.vehicle_type===type)&&(!$('rendered').checked||selectionList(v).some(s=>s.media.length))&&(!q||[v.label,v.description,v.native_name,v.id,v.manufacturer,v.vehicle_type,JSON.stringify(v.tags),JSON.stringify(v.entity),...selectionList(v).map(s=>s.appearance)].join(' ').toLowerCase().includes(q)));
 const key=$('sort').value;rows.sort(key==='name'?(a,b)=>a.label.localeCompare(b.label):(a,b)=>{const get=v=>key==='power'?spec(v).horsepower:key==='mass'?spec(v).physics_mass_kg:top(v).hud_mph;return(get(b)??-Infinity)-(get(a)??-Infinity)||a.id.localeCompare(b.id);});
 page=Math.min(page,Math.max(0,Math.ceil(rows.length/pageSize)-1));$('results').replaceChildren(...rows.slice(page*pageSize,(page+1)*pageSize).map(card));$('status').textContent=`${rows.length.toLocaleString()} vehicle records · ${data.selections.length.toLocaleString()} distinct visual selections`;
 $('page').textContent=`Page ${page+1} of ${Math.max(1,Math.ceil(rows.length/pageSize))}`;$('previous').disabled=page===0;$('next').disabled=(page+1)*pageSize>=rows.length;
}
function detailScript(path){return new Promise((resolve,reject)=>{const script=node('script');script.src=path;script.onload=()=>{const value=globalThis.GHOSTLINE_VEHICLE_DETAIL;delete globalThis.GHOSTLINE_VEHICLE_DETAIL;script.remove();resolve(value);};script.onerror=()=>{script.remove();reject(Error('Native details could not be loaded'));};document.head.append(script);});}
async function route(){
 if(location.hash.startsWith('#/selection/')){
  const generation=++detailGeneration,section=$('detail');section.hidden=false;section.replaceChildren();
  let id;try{id=decodeURIComponent(location.hash.slice('#/selection/'.length));}catch{section.append(node('p','Invalid selection identity'));return;}
  const s=selections.get(id);if(!s){section.append(node('p','Selection not found'));return;}
  const back=node('a','← All vehicles');back.href='#/';const image=visual(s);section.append(back,node('h2',s.appearance||'Native vehicle selection'),node('p',s.id,'identity'),image.box,image.views);
  section.append(button('Copy selection',()=>copy({resource:s.resource,appearance:s.appearance,associated_record_ids:s.records,source:data.sources})),node('p',`${s.resolution} · ${s.preview_state}`));
  for(const id of s.records){const v=records.get(id);if(v){const link=node('a',`${v.label} · ${v.native_name||v.id}`);link.href='#/vehicle/'+encodeURIComponent(id);const p=node('p');p.append(link);section.append(p);}}
  try{const full=await detailScript(s.detail);if(generation===detailGeneration){for(const note of full.recipe?.inspection?.material_notes||[])section.append(node('p',note,'notice'));const d=node('details');d.append(node('summary','Native components, transforms, materials and render provenance'),node('pre',JSON.stringify(full,null,2)));section.append(d);}}catch(e){section.append(node('p',e.message));}
  return;
 }
 const generation=++detailGeneration;const prefix='#/vehicle/';const section=$('detail');section.replaceChildren();section.hidden=!location.hash.startsWith(prefix);if(section.hidden)return;
 let id;try{id=decodeURIComponent(location.hash.slice(prefix.length));}catch{section.append(node('p','Invalid vehicle identity'));return;}
 const v=records.get(id);if(!v){section.append(node('p','Vehicle not found'));return;}
 const back=node('a','← All vehicles');back.href='#/';section.append(back,node('h2',v.label),node('p',v.description||'No localized description available'),card(v));
 const status=node('p','Loading native details…');section.append(status);
 try{const full=await detailScript(v.detail);if(generation!==detailGeneration)return;status.remove();const fold=node('details');fold.dataset.details='';fold.append(node('summary','Native record, relationships and performance provenance'),node('pre',JSON.stringify(full,null,2)));section.append(fold);
  for(const s of selectionList(v)){const d=node('details');d.append(node('summary',`${s.appearance||'Native selection'} · ${s.resolution} · ${s.preview_state}`));let loaded=false;d.ontoggle=async()=>{if(!d.open||loaded)return;loaded=true;try{const value=await detailScript(s.detail);d.append(node('pre',JSON.stringify(value,null,2)));}catch(e){loaded=false;d.append(node('p',e.message));}};section.append(d);}
 }catch(e){status.textContent=e.message;}
}
for(const[id,key]of[['manufacturer','manufacturer'],['type','vehicle_type']])for(const value of [...new Set(data.records.map(v=>v[key]).filter(Boolean))].sort()){const option=node('option',value);option.value=value;$(id).append(option);}
$('filters').onsubmit=e=>e.preventDefault();$('filters').oninput=()=>{page=0;render();};$('previous').onclick=()=>{page--;render();};$('next').onclick=()=>{page++;render();};$('clear-compare').onclick=()=>{compared.clear();save();compare();render();};$('close-image').onclick=()=>$('image-viewer').close();window.addEventListener('hashchange',route);
$('coverage').textContent=`${data.coverage.records.toLocaleString()} records / ${data.coverage.selections.toLocaleString()} selections / ${data.coverage.ready.toLocaleString()} rendered / ${data.coverage.blocked.toLocaleString()} blocked`;$('sources').textContent=JSON.stringify({sources:data.sources,coverage:data.coverage},null,2);compare();render();route();
})();
