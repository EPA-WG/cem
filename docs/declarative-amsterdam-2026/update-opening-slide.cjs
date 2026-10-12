/* Build an in-place native Slides update from a fresh full connector readback.
 * Usage: node update-opening-slide.cjs <raw-presentation.json> <requests.json>
 * Submit with the saved requiredRevisionId; do not reimport the original PPTX.
 */
const fs = require('node:fs');
const [input, output] = process.argv.slice(2);
if (!input || !output) throw new Error('Expected raw presentation and output paths');
const raw = JSON.parse(fs.readFileSync(input, 'utf8'));
const deck = raw.structuredContent || raw;
const slide = deck.slides[0];
if (slide.objectId !== 'p1' || deck.slides.length !== 29) throw new Error('Review changed slide order before applying');
const C = { bg:'F1FEFE', ink:'001010', purple:'6A1B9A', pale:'E1BEE7', trust:'D7E3FF', trustInk:'002F65', teal:'006A6A', neutral:'F1F1EB' };
const requests=[];
const rgb = hex => ({red:parseInt(hex.slice(0,2),16)/255,green:parseInt(hex.slice(2,4),16)/255,blue:parseInt(hex.slice(4,6),16)/255});
const range = {type:'ALL'};
const getText = e => e.shape?.text?.textElements?.map(t=>t.textRun?.content||'').join('') || '';
function replace(id,value) {
  requests.push({deleteText:{objectId:id,textRange:range}},{insertText:{objectId:id,text:value,insertionIndex:0}});
  const original=deck.slides.flatMap(s=>[...s.pageElements,...(s.slideProperties?.notesPage?.pageElements||[])]).find(e=>e.objectId===id);
  const prior=original?.shape?.text?.textElements?.find(e=>e.textRun)?.textRun?.style;
  if(prior){
    const style=Object.fromEntries(['fontFamily','fontSize','foregroundColor','bold','italic','underline'].filter(k=>prior[k]!==undefined).map(k=>[k,prior[k]]));
    requests.push({updateTextStyle:{objectId:id,textRange:range,style,fields:Object.keys(style).join(',')}});
  }
}
function style(id,size,color,bold=false,align='START') {
  requests.push({updateTextStyle:{objectId:id,textRange:range,style:{fontFamily:'Arial',fontSize:{magnitude:size,unit:'PT'},foregroundColor:{opaqueColor:{rgbColor:rgb(color)}},bold,italic:false},fields:'fontFamily,fontSize,foregroundColor,bold,italic'}},
    {updateParagraphStyle:{objectId:id,textRange:range,style:{alignment:align},fields:'alignment'}});
}
function place(id,x,y,w,h){
  const e=slide.pageElements.find(e=>e.objectId===id);
  const unit=e.size.width.unit==='EMU'?12700:1;
  requests.push({updatePageElementTransform:{objectId:id,applyMode:'ABSOLUTE',transform:{scaleX:w/(e.size.width.magnitude/unit),scaleY:h/(e.size.height.magnitude/unit),shearX:0,shearY:0,translateX:x,translateY:y,unit:'PT'}}});
}
function text(id,value,x,y,w,h,size,color,bold=false,align='CENTER'){
  requests.push({createShape:{objectId:id,shapeType:'TEXT_BOX',elementProperties:{pageObjectId:slide.objectId,size:{width:{magnitude:w,unit:'PT'},height:{magnitude:h,unit:'PT'}},transform:{scaleX:1,scaleY:1,shearX:0,shearY:0,translateX:x,translateY:y,unit:'PT'}}}},
    {insertText:{objectId:id,text:value,insertionIndex:0}});
  style(id,size,color,bold,align);
}
function shape(id,type,x,y,w,h,fill,stroke,dash){
  requests.push({createShape:{objectId:id,shapeType:type,elementProperties:{pageObjectId:slide.objectId,size:{width:{magnitude:w,unit:'PT'},height:{magnitude:h,unit:'PT'}},transform:{scaleX:1,scaleY:1,shearX:0,shearY:0,translateX:x,translateY:y,unit:'PT'}}}},
    {updateShapeProperties:{objectId:id,shapeProperties:{shapeBackgroundFill:{solidFill:{color:{rgbColor:rgb(fill)},alpha:1}},outline:{outlineFill:{solidFill:{color:{rgbColor:rgb(stroke)},alpha:1}},weight:{magnitude:1.4,unit:'PT'},...(dash?{dashStyle:'DASH'}:{})}},fields:'shapeBackgroundFill,outline'}});
}
function line(id,x1,y1,x2,y2,color,width=2,arrow=false){
  requests.push({createLine:{objectId:id,lineCategory:'STRAIGHT',elementProperties:{pageObjectId:slide.objectId,size:{width:{magnitude:Math.max(Math.abs(x2-x1),.01),unit:'PT'},height:{magnitude:Math.max(Math.abs(y2-y1),.01),unit:'PT'}},transform:{scaleX:x2<x1?-1:1,scaleY:y2<y1?-1:1,shearX:0,shearY:0,translateX:x1,translateY:y1,unit:'PT'}}}},
    {updateLineProperties:{objectId:id,lineProperties:{lineFill:{solidFill:{color:{rgbColor:rgb(color)},alpha:1}},weight:{magnitude:width,unit:'PT'},...(arrow?{endArrow:'FILL_ARROW'}:{})},fields:'lineFill,weight'+(arrow?',endArrow':'')}});
}
// Keep the user's first slide and the original cover now at position two.
requests.push({updatePageProperties:{objectId:slide.objectId,pageProperties:{pageBackgroundFill:{solidFill:{color:{rgbColor:rgb(C.bg)},alpha:1}}},fields:'pageBackgroundFill'}});
for(const id of ['p1_i2','p1_i3','p1_i5','p1_i6']){
  requests.push({updateTextStyle:{objectId:id,textRange:range,style:{foregroundColor:{opaqueColor:{rgbColor:rgb(C.teal)}}},fields:'foregroundColor'}});
}
requests.push({updateLineProperties:{objectId:'p1_i4',lineProperties:{lineFill:{solidFill:{color:{rgbColor:rgb(C.teal)},alpha:.3}}},fields:'lineFill'}});
replace('p1_i7','CEM');place('p1_i7',44,64,865,53);style('p1_i7',36,C.purple,true);
replace('p1_i8','DNA for structured data');place('p1_i8',44,120,875,61);style('p1_i8',42,C.ink,true);
replace('p1_i9',"A template for today's structure and tomorrow's evolution.");place('p1_i9',46,187,875,37);style('p1_i9',20,C.ink);
place('p1_i10',46,459,430,28);style('p1_i10',18,C.ink,true);
place('p1_i11',579,464,335,24);style('p1_i11',13,C.teal,false,'END');

// One idea becomes several: a small editable branching figure.
const thoughtNodes=[[149,251,18],[118,294,13],[179,294,13],[87,328,10],[135,328,10],[199,328,10]];
[[0,1],[0,2],[1,3],[1,4],[2,5]].forEach(([a,b],i)=>line('teaser_thought_link_'+i,thoughtNodes[a][0],thoughtNodes[a][1],thoughtNodes[b][0],thoughtNodes[b][1],C.trustInk,1.8));
thoughtNodes.forEach(([x,y,r],i)=>shape('teaser_thought_'+i,'ELLIPSE',x-r/2,y-r/2,r,r,i===0?C.trustInk:C.trust,C.trustInk));

// DNA is a visual analogy for inheritance and variation, not a causal origin claim.
const points=21;
const helix=(i,phase)=>[478+30*Math.sin(i/(points-1)*Math.PI*2+phase),239+i/(points-1)*94];
for(let i=0;i<points-1;i++)for(let chain=0;chain<2;chain++){
  const a=helix(i,chain*Math.PI),b=helix(i+1,chain*Math.PI);
  line(`teaser_dna_${chain}_${i}`,...a,...b,chain?C.teal:C.purple,2.4);
}
for(let i=1;i<points;i+=2){const a=helix(i,0),b=helix(i,Math.PI);line('teaser_basepair_'+i,...a,...b,C.teal,1.2);}

// One declared template serves a present state and future variants.
line('teaser_cem_link_now',796,278,752,307,C.purple,1.8);
line('teaser_cem_link_next',796,278,840,307,C.purple,1.8);
shape('teaser_template','ROUND_RECTANGLE',755,238,82,42,C.pale,C.purple);
text('teaser_template_label','template',758,248,76,24,12,C.purple,true);
shape('teaser_now','ROUND_RECTANGLE',717,309,69,33,C.neutral,C.teal);
shape('teaser_next','ROUND_RECTANGLE',805,309,69,33,C.bg,C.purple,true);
text('teaser_now_label','today',720,314,63,23,12,C.ink);
text('teaser_next_label','next',808,314,63,23,12,C.purple);
line('teaser_arrow_1',267,288,351,288,C.teal,1.6,true);
line('teaser_arrow_2',588,288,672,288,C.teal,1.6,true);
[['Thought','One idea gives rise to others.',145,C.trustInk],['DNA','A pattern carried through change.',478,C.teal],['CEM','A template that can evolve.',796,C.purple]].forEach(([title,caption,x,c],i)=>{
  text('teaser_stage_title_'+i,title,x-123,355,246,33,23,c,true);
  text('teaser_stage_caption_'+i,caption,x-135,392,270,47,15,C.ink);
});

const talkingPoints=`00:00–00:45 — Opening metaphor\n\nIn the beginning was a thought. It felt lonely, so it changed—and gave rise to other thoughts. [Pause.]\n\nIn the biological world, DNA offers a vivid parallel: a pattern that life can copy and express, while variation makes evolution possible.\n\nCEM is an attempt to create DNA for structured data: a declarative template for a system's current state and a structure its future versions can build upon.\n\nThat is the ambition behind evolutionary architecture. Let's see what it means for one document.\n\nDelivery: tell this as an opening metaphor. Do not claim that thought literally caused DNA or that CEM is a biological system. The diagram connects ideas about reproduction and change, not a scientific chronology.\n\nVisual theme: canonical CEM light palette; comfort #F1FEFE, comfort-text #001010, creativity-x #6A1B9A, creativity #E1BEE7, trust #D7E3FF, trust-x #002F65, calm-x #006A6A. Slide 2 remains dark.\n\nSource: docs/declarative-amsterdam-2026/opening-teaser.md.`;
replace(slide.slideProperties.notesPage.notesProperties.speakerNotesObjectId,talkingPoints);

// The cloned cover adds a slide, not time: keep all later agenda boundaries.
for(let i=1;i<deck.slides.length;i++){
  const s=deck.slides[i];
  const badge=s.pageElements.find(e=>/^\d{2}\n?$/.test(getText(e)));
  if(badge)replace(badge.objectId,String(i+1).padStart(2,'0'));
}
for(const [idx,oldTime,newTime] of [[1,'00:00–00:45','00:45–01:10'],[2,'00:45–03:00','01:10–03:00']]){
  const s=deck.slides[idx];
  const timer=s.pageElements.find(e=>getText(e).trim()===oldTime);if(timer)replace(timer.objectId,newTime);
  const noteId=s.slideProperties.notesPage.notesProperties.speakerNotesObjectId;
  const note=s.slideProperties.notesPage.pageElements.find(e=>e.objectId===noteId);
  if(note)replace(noteId,getText(note).replace(oldTime,newTime));
}
fs.writeFileSync(output,JSON.stringify({presentationId:deck.presentationId,write_control:{requiredRevisionId:deck.revisionId},requests},null,2)+'\n');
console.log(JSON.stringify({requests:requests.length,slide:slide.objectId,slideCount:deck.slides.length,palette:C,output}));
