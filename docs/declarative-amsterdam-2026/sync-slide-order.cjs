/* Patch the reviewed October 11 native order without reimporting the PPTX.
 * Usage: node sync-slide-order.cjs <raw-presentation.json> <requests.json>
 * The explicit order guard protects subsequent user edits. Submit the saved
 * revision with the requests; re-review this mapping when the deck changes.
 */
const fs = require('node:fs');
const [input, output] = process.argv.slice(2);
if (!input || !output) throw new Error('Expected raw presentation and output paths');
const raw = JSON.parse(fs.readFileSync(input, 'utf8'));
const deck = raw.structuredContent || raw;
const expected = ['p1', 'h36c7c826c4eee100_0_0', ...Array.from({length:20}, (_,i)=>`p${i+2}`), 'h209a9db50e1b8a77_0_0', 'p22', 'p27', 'p23', 'p28', 'p25', 'p26', 'p24'];
if (deck.presentationId !== '1v5JEclFycuXuYk5Mnpwb0alxdKIi30J6XgvLBmawxI4' || JSON.stringify(deck.slides.map(s=>s.objectId)) !== JSON.stringify(expected)) throw new Error('Native slide order changed; review before synchronizing');
const requests = [];
const text = e => (e.shape?.text?.textElements || []).map(t=>t.textRun?.content || '').join('');
const style = e => {
  const prior = e.shape.text.textElements.find(t=>t.textRun)?.textRun.style || {};
  return Object.fromEntries(['fontFamily','fontSize','foregroundColor','bold','italic','underline'].filter(k=>prior[k]!==undefined).map(k=>[k,prior[k]]));
};
// Replace only the specified characters. Preserve the paragraph and restore
// the label's original font and color, including pale text on dark slides.
function replacePrefix(e, before, after) {
  if (!text(e).startsWith(before)) throw new Error(`Unexpected text in ${e.objectId}`);
  requests.push({deleteText:{objectId:e.objectId,textRange:{type:'FIXED_RANGE',startIndex:0,endIndex:before.length}}}, {insertText:{objectId:e.objectId,insertionIndex:0,text:after}});
  const prior = style(e);
  if (Object.keys(prior).length) requests.push({updateTextStyle:{objectId:e.objectId,textRange:{type:'FIXED_RANGE',startIndex:0,endIndex:after.length},style:prior,fields:Object.keys(prior).join(',')}});
}
function createLabel(exemplar, id, page, value) {
  requests.push({createShape:{objectId:id,shapeType:exemplar.shape.shapeType,elementProperties:{pageObjectId:page,size:exemplar.size,transform:exemplar.transform}}}, {insertText:{objectId:id,insertionIndex:0,text:value}});
  const prior = style(exemplar);
  requests.push({updateTextStyle:{objectId:id,textRange:{type:'ALL'},style:prior,fields:Object.keys(prior).join(',')}}, {updateParagraphStyle:{objectId:id,textRange:{type:'ALL'},style:exemplar.shape.text.textElements.find(t=>t.paragraphMarker).paragraphMarker.style,fields:'alignment,lineSpacing,spaceAbove,spaceBelow,indentStart,indentEnd,indentFirstLine,direction,spacingMode'}}, {updateShapeProperties:{objectId:id,shapeProperties:{shapeBackgroundFill:{propertyState:'NOT_RENDERED'},outline:{propertyState:'NOT_RENDERED'},contentAlignment:'MIDDLE'},fields:'shapeBackgroundFill,outline,contentAlignment'}});
}
for (const [i, slide] of deck.slides.entries()) {
  const badges = slide.pageElements.filter(e=>/^\d{2}\n?$/.test(text(e)) && (e.transform?.translateX || 0)>10000000);
  if (slide.objectId==='h209a9db50e1b8a77_0_0' && badges.length===0) continue;
  if (badges.length!==1) throw new Error(`Expected one top-right badge on ${slide.objectId}`);
  const before=text(badges[0]).trim(), after=String(i+1).padStart(2,'0');
  if(before!==after) replacePrefix(badges[0],before,after);
}
const deployment = deck.slides.find(s=>s.objectId==='p21');
const timer = deployment.pageElements.find(e=>text(e).startsWith('49:00–'));
if (text(timer).startsWith('49:00–50:30')) replacePrefix(timer,'49:00–50:30','49:00–50:00');
const notes = deployment.slideProperties.notesPage.pageElements.find(e=>e.objectId===deployment.slideProperties.notesPage.notesProperties.speakerNotesObjectId);
if (text(notes).startsWith('49:00–50:30')) replacePrefix(notes,'49:00–50:30','49:00–50:00');
const added = deck.slides[22];
if (!added.pageElements.some(e=>e.objectId==='dce_promise_number')) {
  createLabel(deployment.pageElements.find(e=>e.objectId==='p21_i3'),'dce_promise_number',added.objectId,'23');
  createLabel(timer,'dce_promise_time',added.objectId,'50:00–50:30');
}
const addedNotes = added.slideProperties.notesPage.pageElements.find(e=>e.objectId===added.slideProperties.notesPage.notesProperties.speakerNotesObjectId);
if (!text(addedNotes).startsWith('50:00–50:30')) {
  requests.push({insertText:{objectId:addedNotes.objectId,insertionIndex:0,text:'50:00–50:30 — DCE promise across execution hosts\n\n'}});
  requests.push({insertText:{objectId:addedNotes.objectId,insertionIndex:text(addedNotes).trimEnd().length+'50:00–50:30 — DCE promise across execution hosts\n\n'.length,text:'\n\nDelivery: use this historical DCE diagram as a bridge from browser composition to supported CEM processing in browser, backend libraries and CLI. The original diagram and Sasha\'s wording above are preserved. Distinguish that processing reach from complete feature parity or security insulation; slides 24–25 explain the enforcement target and current boundaries. Keep this bridge to 30 seconds.\n'}});
}
fs.writeFileSync(output,JSON.stringify({presentation_id:deck.presentationId,write_control:{requiredRevisionId:deck.revisionId},requests},null,2)+'\n');
console.log(JSON.stringify({slides:deck.slides.length,requests:requests.length,output}));
