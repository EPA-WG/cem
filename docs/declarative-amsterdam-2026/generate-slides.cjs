/* Build the INITIAL 28-slide PowerPoint snapshot. The native Google Slides
 * deck contains subsequent additions and Sasha's edits; it is authoritative.
 * Do not reimport this snapshot over the edited deck. Use native updates.
 * Generated binaries belong in dist/.
 * Prerequisites: generate-code-preview.mjs and generate-slide-assets.mjs.
 * Set CEM_SLIDE_NODE_MODULES to a directory containing pptxgenjs if needed.
 */
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '../..');
const libs = process.env.CEM_SLIDE_NODE_MODULES || '/mnt/c/Users/suns/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules';
const PptxGenJS = require(path.join(libs, 'pptxgenjs'));
const pptx = new PptxGenJS();
pptx.layout = 'LAYOUT_WIDE';
pptx.author = 'Sasha Firsov';
pptx.subject = 'Declarative Amsterdam 2026 — CEM platform and document lifecycle';
pptx.title = 'CEM-ML declarative document lifecycle engine';
pptx.company = 'EPA Working Group';
pptx.lang = 'en-GB';
pptx.theme = { headFontFace: 'Arial', bodyFontFace: 'Arial', lang: 'en-GB' };
const OUT = path.join(root, 'dist/declarative-amsterdam-2026/slides');
const ASSETS = path.join(root, 'dist/declarative-amsterdam-2026/slide-assets');
const PREVIEW = path.join(root, 'dist/declarative-amsterdam-2026/code-preview');
fs.mkdirSync(OUT, {recursive:true});
const C = {bg:'F1FAFA', white:'FFFFFF', ink:'16343B', muted:'506A70', purple:'6333A6', pale:'EEE7F7', teal:'087E83', tealPale:'D6EFED', red:'9F334A', line:'C6DCDC', dark:'19333C', gold:'EABB68'};
const ST = pptx.ShapeType;
const records = [];
const resource = 'https://github.com/EPA-WG/cem/discussions/24';
let pageNumber = 0;
let noteSections = [];
const noteFile = path.join(__dirname, 'slide-story.md');
if (fs.existsSync(noteFile)) noteSections = fs.readFileSync(noteFile,'utf8').split(/^## /m).slice(1);

function text(s, value, x,y,w,h,size=22, opt={}) {
  s.addText(value,{x,y,w,h,fontFace:'Arial',fontSize:size,color:C.ink,margin:0,breakLine:false,vertAnchor:'mid',paraSpaceAfterPt:0,...opt});
  if (typeof value==='string') s._record.text.push(value);
}
function box(s,x,y,w,h,fill=C.white,line=C.line,radius=true) {
  s.addShape(radius?ST.roundRect:ST.rect,{x,y,w,h,rectRadius:.14,radius:.14,line:{color:line,width:.8},fill:{color:fill},radius:.12});
}
function line(s,x1,y1,x2,y2,color=C.line,width=1.5,arrow=false) {
  s.addShape(ST.line,{x:x1,y:y1,w:x2-x1,h:y2-y1,line:{color,width,...(arrow?{endArrowType:'triangle'}:{})}});
}
function label(s,t,x,y,w=3,color=C.purple){text(s,t.toUpperCase(),x,y,w,.25,10.5,{bold:true,charSpacing:1.8,color});}
function base(section,title,sub='',time='',dark=false){
  const s=pptx.addSlide(); pageNumber++;
  s.background={color:dark?C.dark:C.bg};
  s._record={number:pageNumber,title,section,time,text:[],images:[]}; records.push(s._record);
  label(s,section,.55,.34,10,dark?'B6DEDE':C.purple);
  text(s,String(pageNumber).padStart(2,'0'),12.18,.32,.58,.3,12,{align:'right',color:dark?'B6DEDE':C.muted});
  if(title) text(s,title,.55,.93,12.25,.9,31,{bold:true,color:dark?C.white:C.ink,fit:'shrink'});
  if(sub)text(s,sub,.57,1.88,12.0,.52,17,{color:dark?'CBE4E4':C.muted});
  line(s,.55,7.06,12.78,7.06,dark?'385A62':C.line,.8);
  text(s,'SASHA FIRSOV  /  DECLARATIVE AMSTERDAM 2026',.55,7.17,8,.18,9,{color:dark?'B6DEDE':C.muted,charSpacing:1});
  text(s,time||'12 NOVEMBER 2026',10.4,7.14,2.35,.25,9,{align:'right',color:dark?'B6DEDE':C.muted});
  return s;
}
function note(s,spoken,caution=''){
  const authority=`Sources: docs/declarative-amsterdam-2026/presentation.md, cem-platform-pitch.md and platform-evidence.md. Resource hub: ${resource}.`;
  const story=noteSections.find(v=>Number(v.match(/^(?:Slide )?(\d+)/i)?.[1])===s._record.number);
  const expanded=story?.split('**Speaker notes:**')[1]?.split('**Sources:**')[0]?.trim();
  s.addNotes(`${s._record.time}\n\n${spoken}${caution?'\n\nPresenter verification note: '+caution:''}\n\n${authority}${expanded?'\n\nExpanded speaking notes:\n'+expanded:''}`);
  s._record.notes=spoken+(caution?'\n'+caution:'');
}
function card(s,x,y,w,h,eyebrow,title,body,fill=C.white){
  box(s,x,y,w,h,fill);label(s,eyebrow,x+.22,y+.22,w-.44);
  text(s,title,x+.22,y+.65,w-.44,.65,23,{bold:true});
  text(s,body,x+.22,y+1.45,w-.44,h-1.65,18,{color:C.muted,breakLine:false});
}
function image(s,name,x,y,w,h,folder=ASSETS){
  const p=path.join(folder,name);if(!fs.existsSync(p))throw new Error(`Missing slide asset: ${p}`);
  const bytes=fs.readFileSync(p),iw=bytes.readUInt32BE(16),ih=bytes.readUInt32BE(20);
  const scale=Math.min(w/iw,h/ih),dw=iw*scale,dh=ih*scale;
  s.addImage({path:p,x:x+(w-dw)/2,y:y+(h-dh)/2,w:dw,h:dh,altText:name.replaceAll('-',' ')});
  s._record.images.push({file:path.relative(root,p),x,y,w,h});
}
function step(s,x,y,w,title,body,fill=C.white){box(s,x,y,w,1.45,fill);text(s,title,x+.17,y+.18,w-.34,.4,21,{bold:true});text(s,body,x+.17,y+.7,w-.34,.5,15.5,{color:C.muted});}
function pill(s,t,x,y,w,fill=C.pale,ink=C.purple){box(s,x,y,w,.44,fill,fill);text(s,t,x+.1,y+.06,w-.2,.29,12,{color:ink,bold:true,align:'center'});}

// 0–7: Value, then architecture.
{
const s=base('CEM  /  DECLARATIVE AMSTERDAM 2026','','','00:00–00:45',true);
text(s,'CEM-ML',.62,1.03,8,.82,51,{bold:true,color:'BDEAE6'});
text(s,'Declarative document\nlifecycle engine',.62,2.02,8.6,1.7,38,{bold:true,color:C.white});
text(s,'Evolutionary architecture for the declarative web stack',.65,4.03,7.5,.95,22,{color:'D0E5E5'});
text(s,'Sasha Firsov',.65,5.64,6,.5,24,{bold:true,color:C.white});
text(s,'12 November 2026 · Amsterdam',.65,6.22,7,.35,16,{color:'B6DEDE'});
['SOURCE','SCHEMA','QUERY','TRANSFORM','DELIVER'].forEach((v,i)=>{
  box(s,9.4,1.12+i*.95,2.55,.62,i===4?'BDEAE6':'284A54','487079');
  text(s,v,9.52,1.29+i*.95,2.3,.25,15,{color:i===4?C.dark:C.white,bold:true,charSpacing:1.2});
  if(i<4)line(s,10.68,1.75+i*.95,10.68,2.05+i*.95,'89C1C3',1.5,true);
});
note(s,'Welcome. CEM enables declarative data services, pipelines and applications. Today we follow one small document from source to a useful result, and use that journey to examine the contracts that make capabilities reusable.','The title matches the published program. The architecture subtitle is thematic framing.');
}
{
const s=base('01 / A useful result','Change the source. See the result.','One login document, followed through its lifecycle.','00:45–03:00');
image(s,'edit-before.png',.55,2.65,6.05,1.22);line(s,6.55,4,7.18,4,C.purple,2,true);
image(s,'login-view.png',7.3,2.57,5.1,3.65);
text(s,'Edit → validate → inspect → convert\n→ query → transform → render',.65,4.42,5.6,1.35,23,{bold:true});
pill(s,'A sign-in view',8.55,6.3,2.6);
note(s,'Begin with the visible result. Point to the heading in the source, then the heading in the view. Invite people to follow on a laptop or predict each change from their seat. We will trace the same document through validation, inspection, conversion, query and rendering. The view is small enough to understand; the lifecycle applies to much larger data services.','The screenshot shows a document view, not authentication, credential storage or session security. Asset evidence records its actual rendering route.');
}
{
const s=base('01 / Why adopt CEM','Make capabilities reusable across systems','Declarative data services, pipelines and applications.','03:00–04:30');
card(s,.55,2.7,3.87,2.65,'FORMAT','Bring the data','Each input and output can use its own supported format.');
card(s,4.73,2.7,3.87,2.65,'LOCATION','Connect the source','Read and publish through extensible, customizable connectors.');
card(s,8.91,2.7,3.87,2.65,'EXECUTION TIER','Choose the host','Use CLI and libraries in browser, server and CI workflows.');
text(s,'Keep processing rules useful when a boundary changes.',.7,5.87,12,0.64,25,{bold:true,color:C.purple});
note(s,'For XML and XSLT developers: keep your document and transformation expertise. For web developers: connect those rules to views. For platform teams: bring the same contracts into services, publishing and CI. A connector is the interface that reads a source or publishes an output; teams can build or adapt one for their own files, services or storage. Format neutrality means shared processing after explicit import and semantic mapping.');
}
{
const s=base('01 / Evolutionary Architecture','Keep useful rules as systems change','Make the boundaries explicit. Protect the contracts with checks.','04:30–07:00');
const a=[['Sources','Formats + locations'],['Schemas','Vocabulary + rules'],['Modules','Query + transform'],['Compose','Inspect + test'],['Deliver','Package + run'],['Evolve','Operate + adapt']];
a.forEach((v,i)=>{step(s,.55+i*2.08,2.83,1.83,v[0],v[1],i===1?C.pale:C.white);if(i<5)line(s,2.4+i*2.08,3.55,2.59+i*2.08,3.55,C.purple,1.5,true);});
box(s,.8,5.12,11.75,.96,C.tealPale,C.tealPale);text(s,'New feed. New vendor. New host.\nKeep the agreed behaviour; adapt the affected boundary.',1.05,5.22,11.25,.75,22,{bold:true,align:'center'});
note(s,'Evolutionary Architecture is controlled change. Follow the full lifecycle: choose sources; pick, extend or create schemas; reuse or develop modules; compose, inspect and test; package and run; operate and evolve. A changed supplier feed should be handled at a defined boundary when the consumer contract can stay stable. Ask: which change is expensive in your system today?','CEM contributes contracts, processing, artifacts and diagnostics; consuming teams still use their own source control, CI, deployment and operational systems.');
}
// 7–17: Familiar comparisons and an audience edit.
{
const s=base('02 / Familiar structure','Keep your markup knowledge','The same heading and field, expressed in two notations.','07:00–09:30');
label(s,'XML / XHTML excerpt',.65,2.62,5.8);label(s,'CEM-ML excerpt',6.94,2.62,5.8);
image(s,'markup-xml.png',.55,3.03,5.95,2.72);image(s,'markup-cem.png',6.83,3.03,5.95,2.72);
text(s,'Names · attributes · content',.7,6.19,12,.45,25,{bold:true,color:C.purple,align:'center'});
note(s,'Start with the familiar side. Ask the audience to find the heading, the label and the input. Move to the CEM-ML side and find the same information. Introduce only the braces, @ attributes and content separator needed to read this excerpt. Namespace context belongs to the complete document.','These are labelled excerpts. The XML export and any explicit namespace repair are recorded in slide-assets/evidence; do not imply arbitrary lossless conversion.');
}
{
const s=base('02 / Read the document','A small vocabulary for the whole journey','Structure remains visible while tools change around it.','09:30–12:00');
image(s,'markup-cem.png',.63,2.72,6.6,3.25);
[['Namespace','Which vocabulary owns a name'],['Node','An element and its children'],['@ attribute','A named value on the node'],['| content','Text or nested structure']].forEach((v,i)=>{
  text(s,v[0],7.74,2.8+i*.85,4.2,.38,21,{bold:true,color:C.purple});text(s,v[1],7.74,3.2+i*.85,4.65,.3,15.5,{color:C.muted});
});
note(s,'A namespace identifies the vocabulary. A node has a name; attributes attach named values; the content separator introduces text or child structure. Point back to the complete login source in the resources. Do not turn this into a grammar lesson: participants only need enough notation to make the next edit.');
}
{
const s=base('02 / Your first edit','Make one change','Change the heading from “Sign in” to “Welcome back”.','12:00–15:00');
label(s,'Before',.68,2.71,3);image(s,'edit-before.png',.6,3.05,12.1,1.03);
label(s,'After',.68,4.36,3);image(s,'edit-after.png',.6,4.7,12.1,1.03);
text(s,'Predict the visible result. Then run it.',.65,6.23,12,.4,24,{bold:true,color:C.purple});
note(s,'Pause for the room. Ask participants to change only the heading text, or tell a neighbour what will change. Keep the identifier and accessibility reference intact. Show the checkpoint if a local setup does not work. The aim is a confident first edit, not speed. Ask which part of the document remained stable.');
}
{
const s=base('02 / Shared schema contracts','Pick, extend, or create your own schemas','Define vocabulary, types and validation rules.','15:00–17:00');
card(s,.55,2.68,3.87,2.9,'01','Pick','Start with an existing schema and its agreed vocabulary.');
card(s,4.73,2.68,3.87,2.9,'02','Extend','Add constraints or specialize a contract for your use case.',C.pale);
card(s,8.91,2.68,3.87,2.9,'03','Create','Describe the vocabulary and rules your own domain needs.');
text(s,'Deliver formatters and colorers with the schema.',.7,6.08,12,.58,24,{bold:true,color:C.purple});
note(s,'Give all three choices equal weight: pick, extend, or create your own schemas. Schemas help development and operations, not only a final validation gate. They define expected vocabulary, types, constraints and useful diagnostics. Deliver pretty and tabular formatters and HTML or terminal colors alongside those definitions.','A new lexical syntax also requires a parser/import adapter. A schema package does not alone parse any invented syntax.');
}
// 17–35: Error, inspection and conversion.
{
const s=base('03 / Diagnose','Catch the mistake at its source','A teaching schema requires an explicit form method.','17:00–20:00');
image(s,'validation-before.png',.65,2.5,11.9,.83);
box(s,.65,3.6,12.02,1.67,C.white,C.line);
text(s,'demo.login.method_required',.9,3.79,11.45,.41,23,{bold:true,color:C.red});
text(s,'Login form must declare its method.',.9,4.38,11.45,.46,23);
const q=['What failed?','Where is the source?','Which rule applies?'];q.forEach((v,i)=>{pill(s,v,.75+i*4.2,5.8,3.85);});
note(s,'Remove only @method=post from the form. The teaching schema requires that field. The real native run reports demo.login.method_required and one error, with exit status 1. The slide quotes the diagnostic message prefix; its full field-contract detail is retained in the report. Ask the audience to distinguish a syntax failure from a schema violation.','The diagnostic top-level location is 1:1 in this binary; the retained source frame points to the form. Do not pretend the header gives a precise token location. Three baseline unresolved-reference warnings remain in both invalid and repaired runs.');
}
{
const s=base('03 / Repair','Repair the cause. Run the check again.','Use the source location to make one deliberate correction.','20:00–23:00');
step(s,.65,3,3.55,'Locate','Follow the diagnostic',C.pale);line(s,4.24,3.72,4.67,3.72,C.purple,2,true);
step(s,4.89,3,3.55,'Repair','Restore the schema contract');line(s,8.48,3.72,8.93,3.72,C.purple,2,true);
step(s,9.1,3,3.55,'Recheck','Confirm the error is gone',C.tealPale);
image(s,'validation-after.png',.75,4.72,11.9,.84);
text(s,'Repaired: exit 0 · zero errors · three baseline warnings',.75,6.04,11.9,.57,21,{bold:true,color:C.purple});
note(s,'Let participants propose the smallest repair before revealing it. Apply the exact repaired fixture and rerun the same command. Compare the diagnostic counts and exit status; do not switch to a weaker validation mode. Connect the exercise to SDLC: the check can run during authoring and again in the build or service pipeline.','The reproducible asset evidence is authoritative for the error code, location, repair and exit statuses. This slide is an instruction sequence, not a fabricated CLI transcript.');
}
{
const s=base('03 / Inspect','Inspect the meaning behind the syntax','Find the same node in the source and the typed structure.','23:00–26:00');
image(s,'inspect.png',.57,2.7,12.16,3.22);
pill(s,'name = input',.7,6.18,3.3);pill(s,'source = login.xml',5.0,6.18,3.3);pill(s,'bytes = 252…330',9.28,6.18,3.3);
note(s,'Point to one node in the inspection and find it in the source. Explain the difference between source spelling and retained structure. Inspection makes the engine’s interpretation visible. Use the documented typed CEM-tree view and tabular output, with an explicit JSON boundary only when requested.','This is a real native inspection excerpt. Compacting for a slide must not invent fields or remove a status that changes its meaning.');
}
{
const s=base('03 / Format and report','Make structure readable','Adopt pretty and tabular formatters, color output in HTML and terminals.','26:00–29:00');
image(s,'markup-cem.png',.7,2.6,7.25,3.24);
card(s,8.43,2.65,4.28,3.17,'SCHEMA DELIVERY','Readable by design','Keep formatting, token colors and diagnostics alongside the vocabulary.',C.pale);
text(s,'Inspect the structure. Read the report. Follow the source.',.7,6.12,12,.49,23,{bold:true,color:C.purple});
note(s,'These examples use the real CEM tabular formatter and semantic HTML colorer. Colors distinguish token roles; layout exposes structure. Use the report to separate warnings, errors and output artifacts. Good presentation is part of a schema’s delivery, useful in terminals, documentation and editors.','The local native binary formats the source. The existing CEM browser/WASM HTML colorer supplies the approved colors because the direct native HTML color route currently returns plain text.');
}
{
const s=base('03 / Convert','Change representation. Preserve agreed meaning.','Import an XML export of the login through the same lifecycle.','29:00–33:00');
step(s,.65,2.83,3.55,'XML','A supported input');line(s,4.24,3.55,4.67,3.55,C.purple,2,true);
step(s,4.89,2.83,3.55,'Typed model','Structure + values',C.pale);line(s,8.48,3.55,8.93,3.55,C.purple,2,true);
step(s,9.1,2.83,3.55,'Explicit output','Choose the representation');
text(s,'Compare the heading, field names and attributes.',.7,5.15,12,.63,25,{bold:true});
text(s,'Spelling and mappings can change. State what must stay equivalent.',.7,5.95,12,.56,20,{color:C.muted});
note(s,'Use the verified CEM-to-XML export and its native XML import. Restore the source namespace bindings explicitly, as recorded in the generated script. Compare named semantic fields, not byte equality. Source attribution changes with the source; the domain expectation can remain stable. Conversion changes representation; a transformation can derive a new result.','The public tutorial still needs the HTML-alternate-input checkpoint. Current query assets use an XML export of the canonical CEM source with explicit namespace declarations restored. Do not present an unverified direct CEM or HTML query route.');
}
{
const s=base('03 / Beyond one document','Multiple inputs. Multiple useful outputs.','Each source and destination can use its own format.','33:00–35:00');
step(s,.65,2.7,3.25,'XML catalog','Product descriptions');step(s,.65,4.57,3.25,'CSV stock','Availability');
line(s,3.9,3.42,4.8,4.02,C.purple,1.7,true);line(s,3.9,5.29,4.8,4.57,C.purple,1.7,true);
box(s,4.85,3.6,3.15,1.45,C.pale,C.pale);text(s,'Shared domain rules',5.07,3.83,2.75,.53,22,{bold:true,align:'center'});
['YAML partner feed','JSON service response','Availability view'].forEach((v,i)=>{line(s,8.02,4.33,9.0,3.1+i*1.34,C.purple,1.5,true);box(s,9.08,2.72+i*1.34,3.64,.78);text(s,v,9.25,2.88+i*1.34,3.3,.45,19,{bold:true});});
pill(s,'Companion scenario',.65,6.36,2.85);
note(s,'The small login exercise demonstrates a lifecycle, not the limit of the platform. Consider a supplier catalog in XML and a stock feed in CSV. Explicit domain rules reconcile their records and produce a YAML feed, a JSON response and a view. Connectors handle locations; import/export adapters handle representations.','The checked-in Supplier pipeline project composes this scenario in the native CLI and Studio browser worker. Its authored rules cover key matching, numeric conversion and missing/duplicate records. Additional service and portal embedding hosts remain future demonstrations.');
}
// 35–47: Queries, transformation, provenance and embedding.
{
const s=base('04 / Query','Ask a familiar question','Select the input fields in the login document.','35:00–37:00');
label(s,'XPath',.7,2.63,5.7);label(s,'CEM-QL',6.94,2.63,5.7);
image(s,'query-xpath.png',.6,3.4,5.95,1.18);image(s,'query-cemql.png',6.83,3.0,5.95,2.68);
text(s,'Same two fields',.7,6.13,3.2,.45,22,{bold:true,color:C.purple});pill(s,'email',4.38,6.12,2.9);pill(s,'password',7.58,6.12,2.9);
note(s,'Read the XPath query first and ask for the expected result. Read the CEM-QL equivalent, then reveal the recorded result: the two login input fields. The value is preserving your question as you use shared processing interfaces. Make the input format and namespace bindings explicit.','These exact expressions were verified against the recorded XML input. They do not establish full language equivalence, full XPath conformance or support for direct queries against every format in this binary.');
}
{
const s=base('04 / Transform','Declare the transformation','Select data, evaluate expressions, and produce a view.','37:00–39:00');
step(s,.65,2.87,3.55,'Query','Which source values?');line(s,4.25,3.59,4.67,3.59,C.purple,2,true);
step(s,4.89,2.87,3.55,'Expression','What derived value?',C.pale);line(s,8.48,3.59,8.93,3.59,C.purple,2,true);
step(s,9.1,2.87,3.55,'Template','What output structure?');
text(s,'Keep modules reusable. Make dependencies explicit.',.65,5.2,12,.7,27,{bold:true,color:C.purple});
text(s,'Reuse CEM-QL and a defined XSLT profile.',.65,6.07,12,.4,20,{color:C.muted});
note(s,'Separate the query, expression and template in the prepared walkthrough. A query selects source data; an expression derives a value; a template defines the result structure. Ask the audience to predict one output change, then use the verified checkpoint. Keep XSLT experience in the story through the supported compilation/runtime profile.','This explanatory graph is not a transcript of an unverified template execution. The exact login template and source-byte transformation trace still need an end-to-end rehearsal; see deck-delivery.md. Never invent a working command while presenting.');
}
{
const s=base('04 / Provenance','Follow the result back to its source','Source identity and transformation context belong in the lifecycle.','39:00–41:00');
step(s,.65,2.89,3.55,'Source','Document + location',C.tealPale);line(s,4.24,3.61,4.68,3.61,C.purple,2,true);
step(s,4.89,2.89,3.55,'Rule','Query or transformation',C.pale);line(s,8.48,3.61,8.93,3.61,C.purple,2,true);
step(s,9.1,2.89,3.55,'Result','Value + producing stage');
line(s,10.84,4.74,2.43,4.74,C.teal,2,true);
text(s,'Where did this value come from?',.7,5.28,12,.64,29,{bold:true,color:C.purple});
text(s,'Inspect source spans, diagnostics and the operation report.',.7,6.11,12,.4,20,{color:C.muted});
note(s,'Start from a value the audience can see. Trace it back to the selected source node and the rule that produced it. Show the actual source location from the recorded query/report. Explain why the same link is valuable in a service or CI failure. A graph makes stages visible; provenance connects results to their origins.','The diagram explains the contract. Use only the source positions and transform trace actually exposed by the chosen operation; a full generated-login source map has not been proven by the syntax-coloring preview.');
}
{
const s=base('04 / Render','Render into the document you can inspect','Declarative data, events, queries and templates meet in <cem-element>.','41:00–44:00');
image(s,'login-view.png',.64,2.62,5.4,3.95);
text(s,'Static HTML preview',.7,6.66,5.3,.24,12,{color:C.muted,align:'center'});
text(s,'Data → query → template → light DOM',6.62,2.93,5.96,1.14,27,{bold:true,color:C.purple});
text(s,'Use ordinary browser inspection.\nKeep semantic markup and public styling hooks.',6.62,4.5,5.78,1.34,22);
note(s,'Show the edited view and inspect its browser DOM. Explain how cem-element joins declarative data, event handling, queries and templates around the reusable document machinery. Light DOM allows ordinary browser inspection and host integration. The component runtime is distinct from a static HTML screenshot.','The illustrated sign-in is a static view generated from the documented login source route. The complete cem-element interactive login checkpoint remains to be rehearsed; do not use this still image as proof of that path.');
}
{
const s=base('04 / Adoption','Embedding applications','Put a capability inside the workflow that needs it.','44:00–47:00');
box(s,.63,2.69,12.05,1.43,C.pale,C.pale);
text(s,'“Put a supplier’s availability check inside a buyer’s ordering workflow.”',.98,2.92,11.31,.91,25,{italic:true,color:C.purple});
text(s,'“Put a specialist transformation inside the pipeline that needs it.”',.78,4.44,11.9,.84,23,{italic:true});
text(s,'Provider maintains the capability.\nHost supplies context, permitted actions and visual integration.',.78,5.7,11.9,.89,22,{bold:true});
note(s,'As AI accelerates software creation, more capabilities compete for a place in existing workflows. Providers gain reach when their product fits inside another product; hosts gain functionality without rebuilding it. Shared host privileges make integration harder to assess and govern. Runtime insulation aims to ease that work while keeping each provider responsible for its capability. Users can stay in their workflow instead of repeatedly navigating and re-entering data.','This is a market thesis and architectural direction, not a measured reduction in adoption costs. Browser sandboxing already exists; CEM’s ambition is integrated controls around declarative scopes and pipelines.');
}
// 47–54: One platform, explicit controls, performance mechanisms.
{
const s=base('05 / Platform map','Find your place in the platform','One processing foundation, several ways to adopt it.','47:00–49:00');
box(s,.65,5.4,12.03,.97,C.pale,C.pale);text(s,'Rust engine  +  schema packages',.87,5.57,11.6,.45,27,{bold:true,color:C.purple,align:'center'});
['Native CLI / library','Node / browser WASM','Worker hosts'].forEach((v,i)=>{box(s,.65+i*4.1,4.08,3.83,.87);text(s,v,.83+i*4.1,4.26,3.47,.4,20,{bold:true,align:'center'});line(s,2.55+i*4.1,5.36,2.55+i*4.1,4.98,C.purple,1.4,true);});
['<cem-element>','Consumer Semantic Theme','CEM Components'].forEach((v,i)=>{box(s,.65+i*4.1,2.69,3.83,.88,C.tealPale,C.tealPale);text(s,v,.8+i*4.1,2.84,3.53,.52,i===1?17.5:20,{bold:true,align:'center'});});
text(s,'CEM Studio · workbench in development        CEM Site · docs and examples',.72,6.56,12,.28,15.5,{color:C.muted,align:'center'});
note(s,'Map the exercise onto the platform. Rust and schema packages provide the core contracts. Native and WASM builds expose CLI and library services to Node, browser and worker hosts. cem-element connects document processing to declarative browser behaviour. Consumer Semantic Theme provides semantic styling; CEM Components provides reusable controls. CEM Studio is the workbench in development, and CEM Site is the documentation and example surface.','Module status and migration readiness vary. Studio is not ready for participant use. Browser command service uses a dedicated worker; a worker API does not imply every graph stage runs in parallel.');
}
{
const s=base('05 / Deployment and extension','Put processing where the work happens','Connectors read sources and publish to destinations.','49:00–50:30');
[['Linux native CLI','Batch, publishing, validation, CI'],['Library','Services, editors and developer tools'],['Node / browser WASM','Supported operations in each host'],['Declarative components','Interactive views in host workflows']].forEach((v,i)=>{
box(s,.65,2.57+i*.9,12.03,.72,i%2?C.white:C.tealPale,i%2?C.line:C.tealPale);text(s,v[0],.9,2.72+i*.9,4.12,.39,21,{bold:true});text(s,v[1],5.22,2.72+i*.9,7.1,.39,19,{color:C.muted});});
text(s,'Build, extend and customize connectors for your own systems.',.72,6.45,11.97,.38,20,{bold:true,color:C.purple});
note(s,'Choose the adoption surface that removes useful work today. A native CLI fits scheduled jobs and CI. A library fits a service or an editor. Node and browser WASM provide supported processing with host-specific I/O. Components provide interaction. Connectors are extension interfaces for reads and publication; credentials, permissions and transport remain explicit host responsibilities.','Do not claim automatic deployment, a built-in HTTP server, a distributed scheduler or identical I/O on every host. Public installation artifacts and clean-install instructions need their own checks.');
}
{
const s=base('05 / Multi-vendor scopes','Insulate each vendor’s contribution','Requests → host grants → runtime enforcement','50:30–52:30');
box(s,.65,2.75,7.1,3.68,C.white,C.teal);label(s,'HOST POLICY + RESOURCE BUDGET',.92,3,6.5,C.teal);
box(s,1.02,3.54,3,2.37,C.pale,C.purple);text(s,'Vendor A',1.28,3.76,2.46,.42,23,{bold:true,color:C.purple});
box(s,1.4,4.55,2.27,.93,C.white,C.purple);text(s,'Included scope',1.53,4.82,2.02,.35,16,{bold:true});
box(s,4.39,3.54,2.97,2.37,C.tealPale,C.teal);text(s,'Vendor B',4.67,3.76,2.43,.42,23,{bold:true,color:C.teal});text(s,'Explicit inputs\nand permitted effects',4.67,4.55,2.38,.9,17);
['Memory','Storage','CPU','Network','DOM','CSS'].forEach((v,i)=>{pill(s,v,8.28+(i%2)*2.31,2.94+Math.floor(i/2)*.83,2.08,i<4?C.pale:C.tealPale,i<4?C.purple:C.teal);});
text(s,'Nested scopes stay within\nparent authority and budgets.',8.32,5.74,4.26,.65,18,{bold:true});
text(s,'Architecture target · current controls are partial',.74,6.61,12,.27,13,{color:C.muted});
note(s,'Security insulation must be an enforced runtime boundary. Give each vendor explicit inputs and permitted actions. The host grants authority; runtime mechanisms enforce it. Included scopes must not increase authority or evade ancestor budgets. Memory, Storage, CPU and Network matter in processing; DOM and CSS add browser effect channels. The benefit sought is useful composition with bounded exposure and disruption.','The integrated six-layer model is an architectural target. Current controls have distinct threat models and do not provide full hostile-vendor confinement. Native plugins run with host privileges; resolver defaults are passthrough. CSS ownership is not a complete security sandbox. See appendix for per-layer boundaries.');
}
{
const s=base('05 / Performance','Make performance a lifecycle property','Reduce repeated work. Bound resource use. Preserve useful state.','52:30–54:00');
card(s,.55,2.63,3.87,2.8,'DATA','Retain typed structure','Avoid unnecessary parse / serialize cycles between compatible stages.');
card(s,4.73,2.63,3.87,2.8,'EXECUTION','Bound independent work','Use explicit dependencies, worker limits and suitable progressive stages.',C.pale);
card(s,8.91,2.63,3.87,2.8,'DELIVERY','Resume component state','Adopt compatible server output and load declarations when needed.');
text(s,'Transformation graphs connect data pipelines and web asset assembly.',.65,5.86,12,.52,21,{bold:true,color:C.purple});
text(s,'Host and workload determine the benefit. Full bundler parity is a goal.',.65,6.49,12,.32,15.5,{color:C.muted});
note(s,'For consumers, aim for earlier useful results, responsive interaction and preserved state. For teams, aim for less repeated processing and predictable capacity. Retained typed data avoids some conversions; bounded execution schedules independent work; compatible component SSR/resume can preserve rendered output and state. A transformation graph also makes web asset dependencies explicit.','No benchmark or monetary saving is claimed. Browser progressive AST consumption and all-stage streaming are not complete. WASM projection routing is sequential. Component SSR/resume has bounded support; whole-page CLI SSR and automatic viewport hydration are not established. Modern bundler parity remains a goal.');
}
{
const s=base('06 / Questions and next steps','Bring one useful integration','Bring your data. Keep your expertise. Compose the capability you need.','54:00–60:00',true);
['A format boundary to simplify','A processing rule to reuse','A partner capability to embed'].forEach((v,i)=>{text(s,String(i+1).padStart(2,'0'),.72,2.92+i*.85,.6,.5,22,{color:'BDEAE6',bold:true});text(s,v,1.57,2.9+i*.85,10.7,.57,27,{color:C.white,bold:true});});
text(s,'Resources and updates',.76,5.78,11.8,.48,22,{color:'BDEAE6',bold:true});
text(s,'github.com/EPA-WG/cem/discussions/24',.76,6.37,9.0,.4,19,{color:C.white,hyperlink:{url:resource}});
if(fs.existsSync(path.join(OUT,'resources-qr.png')))image(s,'resources-qr.png',10.75,5.18,1.63,1.63,OUT);
note(s,'Keep the final six minutes for questions and use cases. Ask each audience group what they would bring: a source or schema, a reusable transform, or an embedded capability. Start a bounded experiment with explicit input and expected output, then make a meaningful format, vendor or host change and see what remains reusable. Use discussion 24 for the deck, examples, releases and Studio access as they become available.');
}
// Reference appendix. Not part of the 60-minute speaking run.
{
const s=base('Appendix / Catalog','32 schema packages','Data formats, languages and platform contracts; support varies by profile.','REFERENCE');
const groups=[['Data and markup · 9','csv · html · json · markdown · mathml\nsvg · xhtml · xml · yaml'],['Query, style, transformation · 5','css · css-selector · scss\nxpath · xslt'],['External schema resources · 2','json-schema · relax-ng'],['CEM authoring and schemas · 7','cem-ml · cem-ql · cem-native-template\ncem-element-template · cem-transform\nschema · schema-package'],['Platform contracts · 6','cem-data-island · cli-command\nmodule-map · module-map-v2\nmodule-map-v3 · studio-project'],['Structural projections · 3','cem-ast-projection\ncem-dom-projection\ncem-events-projection']];
groups.forEach((v,i)=>{const x=.6+(i%3)*4.19,y=2.58+Math.floor(i/3)*1.92;box(s,x,y,3.96,1.68);text(s,v[0],x+.17,y+.15,3.62,.32,15.5,{bold:true,color:C.purple});text(s,v[1],x+.17,y+.62,3.62,.86,14.5,{color:C.ink});});
text(s,'Includes internal contracts and three module-map revisions.',.72,6.63,12,.26,13,{color:C.muted});
note(s,'The count is 32 package manifests, not 32 fully implemented external formats. Present capability depth, not just a logo wall. Point to platform-evidence.md for manifest links and profile limits. JSON Schema and RELAX NG package presence does not establish complete instance validation.');
}
{
const s=base('Appendix / Compatibility','Reuse expertise within explicit profiles','Start with the operation and input you can verify.','REFERENCE');
[['XML / (X)HTML','Import, document structure and explicit delivery mappings'],['XPath / CEM-QL','Verified equivalent queries on the selected input'],['XSLT','A defined compilation and runtime profile'],['Formats + locations','Adapters extend representation; connectors extend access'],['Hosts','Native, Node and browser capabilities differ']].forEach((v,i)=>{text(s,v[0],.72,2.7+i*.7,3.5,.42,19,{bold:true,color:C.purple});text(s,v[1],4.32,2.7+i*.7,8.3,.43,18);line(s,.7,3.22+i*.7,12.66,3.22+i*.7,C.line,.7);});
note(s,'Use current owning package contracts and verified examples. XPath and CEM-QL are not generally interchangeable; XSLT support is bounded. CSV values need explicit typing when the domain needs numbers. The YAML query profile excludes some features. Connectors and format adapters solve separate boundaries. Native and browser resource access differ.');
}
{
const s=base('Appendix / Enforcement','Six layers: foundations and boundaries','A control is only as strong as the effect paths it covers.','REFERENCE');
const rows=[['Memory','Accounted allocations, inherited ceilings','Complete per-vendor private-memory isolation'],['Storage','Host ports, explicit export and redaction','Every-path authority and namespace quotas'],['CPU','Bounded workers, cancellation, deadlines','Preemption of arbitrary in-process code'],['Network','Resolver policy before configured access','All-path egress, redirects and credentials'],['DOM','Owned ranges, inert state, workers','Hostile same-page-script confinement'],['CSS','Native @scope and ownership contracts','Containment of every style/resource effect']];
label(s,'Layer',.7,2.61,1.3);label(s,'Current foundations',2.22,2.61,4.85);label(s,'Boundary still to establish',7.8,2.61,4.85);
rows.forEach((v,i)=>{const y=3.03+i*.55;text(s,v[0],.7,y,1.25,.36,16,{bold:true,color:C.purple});text(s,v[1],2.22,y,5.16,.4,14.5);text(s,v[2],7.8,y,4.86,.4,14.5,{color:C.muted});line(s,.7,y+.46,12.66,y+.46,C.line,.6);});
note(s,'Do not infer full insulation from naming scopes. Allocation accounting is not private heap isolation. CSS ownership is not a sandbox. Native plugins execute in process with host privileges and post-call timeout checks do not preempt arbitrary code. Resolver policy defaults to passthrough; a restrictive policy must be configured. Use observable allowed and denied cases per channel for future proof.');
}
{
const s=base('Appendix / Reproducibility','Follow the source and the evidence','The CLI and code preview work independently of Studio.','REFERENCE');
[['Source','examples/cem-ml/login.cem'],['Code presentation','Native tabular formatter → CEM HTML colorer'],['Asset evidence','slide-assets / evidence and command transcript'],['Resources','github.com/EPA-WG/cem/discussions/24']].forEach((v,i)=>{label(s,v[0],.7,2.7+i*.8,3.0);text(s,v[1],3.42,2.63+i*.8,9.12,.48,20,{hyperlink:i===3?{url:resource}:undefined});});
text(s,'Use the recorded checkpoints if a local setup fails.',.72,6.3,12,.43,23,{bold:true,color:C.purple});
note(s,'Generation sources live under docs/declarative-amsterdam-2026. The approved preview preserves native formatted source and CEM semantic colors. Generated binaries live under dist/declarative-amsterdam-2026. Use deck-delivery.md for exact versions, sources, verification, artifact paths and remaining rehearsal gaps. Studio is not required.','Public clean-install instructions, the complete interactive template/cem-element walkthrough, full generated-result transformation trace and timed rehearsal remain separate preparation tasks. Do not tell participants those are verified by deck creation.');
}

async function main(){
  if(records.length!==28)throw new Error('Unexpected slide count');
  fs.writeFileSync(path.join(OUT,'slide-content.json'),JSON.stringify(records,null,2)+'\n');
  await pptx.writeFile({fileName:path.join(OUT,'cem-declarative-amsterdam-2026.pptx')});
  console.log(JSON.stringify({slides:records.length,notes:records.filter(r=>r.notes).length,output:OUT}));
}
main().catch(e=>{console.error(e);process.exitCode=1});
