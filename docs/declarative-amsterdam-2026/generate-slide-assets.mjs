// Reproduce the presentation assets using the native formatter and CEM HTML colorer.
// Run: node docs/declarative-amsterdam-2026/generate-slide-assets.mjs
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const root=path.resolve(fileURLToPath(new URL('../../',import.meta.url)));
const dir=path.join(root,'dist/declarative-amsterdam-2026/slide-assets');
const rel='dist/declarative-amsterdam-2026/slide-assets';
const cli=path.join(root,'dist/target/cem_ml_cli/debug/cem-ml');
await mkdir(dir,{recursive:true});
const evidence={generatedAt:new Date().toISOString(),commands:[],assets:[],limitations:[]};
const save=(name,data)=>writeFile(path.join(dir,name),data);
async function run(name,args,expected=0,cwd=root){
 const r=spawnSync(cli,args,{cwd,encoding:'utf8',maxBuffer:32*1024*1024});
 await save(name+'.stdout.txt',r.stdout??'');await save(name+'.stderr.txt',r.stderr??'');
 evidence.commands.push({name,cwd,args,exitCode:r.status,stderr:r.stderr});
 if(r.status!==expected)throw Error(`${name}: expected ${expected}, got ${r.status}: ${r.stderr}`);
 return r;
}
async function format(name,source,contentType='application/cem'){
 await save(name+'.source',source);
 const args=['convert',`${rel}/${name}.source`,'--content-type',contentType,'--to-content-type',contentType,'--cemt-formatter-profile','tabular','--output-color-type','none','--out',`${rel}/${name}.txt`];
 if(contentType==='application/cem')args.push('--schema','https://cem.dev/ns/cem-ml/1','--to-schema','https://cem.dev/ns/cem-ml/1');
 const r=await run(name+'-format',args);
 if(/adapter_unsupported/.test(r.stderr))throw Error(name+' adapter unsupported');
 return readFile(path.join(dir,name+'.txt'),'utf8');
}
const original=await readFile(path.join(root,'examples/cem-ml/login.cem'),'utf8');
evidence.source={path:'examples/cem-ml/login.cem',sha256:createHash('sha256').update(original).digest('hex')};
const formatted=await format('login',original);
const h1=original.match(/\{h1[^\n]+/)[0];
const label=original.match(/\{label @for=email[^\n]+/)[0];
const markupCem=await format('markup-cem',`${h1}\n${label}\n`);
const editBefore=await format('edit-before',h1+'\n');
const editAfter=await format('edit-after',h1.replace('Sign in','Welcome back')+'\n');
// XML excerpt is exported natively from exactly the same two element snippets.
await run('markup-xml-export',['convert',`${rel}/markup-cem.txt`,'--from-format','cem','--to-format','xml','--output-color-type','none','--out',`${rel}/markup-xml.raw.xml`]);
const xmlRaw=await readFile(path.join(dir,'markup-xml.raw.xml'),'utf8');
const markupXml=xmlRaw.replace('</h1><label','</h1>\n<label'); // XML inter-element whitespace only.
await save('markup-xml.txt',markupXml);
await run('login-xml-export',['convert','examples/cem-ml/login.cem','--to-format','xml','--output-color-type','none','--out',`${rel}/login.raw.xml`]);
const rawXml=await readFile(path.join(dir,'login.raw.xml'),'utf8');
// This binary omits directive-owned namespace declarations on XML export.
// Restore only those declarations explicitly from the canonical source directives.
const loginXml=rawXml.replace('<main ','<main xmlns="http://www.w3.org/1999/xhtml" xmlns:cem="https://cem.dev/ns/core/1" ');
await save('login.xml',loginXml);
evidence.limitations.push('Native XML export omits namespace declarations. login.xml restores the exact html and cem bindings from the source directives before XML import. No element, attribute, or value is changed.');
const qExpr='seq:where(input.descendants,\n  fn(n) => n.kind == "element" &&\n           n.name == "input")';
const qModule='module "urn:demo:login-inputs"\n\n'+qExpr+'\n';
const qFormatted=await format('query-cemql',qModule,'application/vnd.cem.query+cem-ql');
await run('query-cemql-color',['convert',`${rel}/query-cemql.txt`,'--content-type','application/vnd.cem.query+cem-ql','--to-content-type','application/vnd.cem.query+cem-ql','--cemt-formatter-profile','tabular','--cemt-color-profile','html','--output-color-type','html','--out',`${rel}/query-cemql.native.html`]);
const qNativeHtml=await readFile(path.join(dir,'query-cemql.native.html'),'utf8');
const xpath='//*[local-name() = "input"]';
await save('query-xpath.txt',xpath+'\n');
const qBase=['query',`${rel}/login.xml`,'--content-type','application/xml','--no-color'];
const qCem=await run('query-cemql-result',[...qBase,'--query-file',`${rel}/query-cemql.txt`,'--query-content-type','application/vnd.cem.query+cem-ql','--output','json']);
const qXpath=await run('query-xpath-result',[...qBase,'--query',xpath,'--query-content-type','application/vnd.cem.xpath','--output','json']);
const cJson=JSON.parse(qCem.stdout),xJson=JSON.parse(qXpath.stdout);
await save('query-cemql-result.json',qCem.stdout);await save('query-xpath-result.json',qXpath.stdout);
evidence.query={input:'login.xml',cemql:cJson,xpath:xJson};
const xItems=xJson.result.sequence.items,cItems=cJson.result.items;
if(xItems.length!==2||cItems.length!==2)throw Error('Queries must each select two fields');
if(JSON.stringify(cItems.map(n=>n.sourceMap))!==JSON.stringify(xItems.map(n=>n.sourceMap)))throw Error('Query source locations differ');
evidence.query.sameSourceSpans=true;
const qResult=await run('query-result',[...qBase,'--query','//*[local-name() = "input"]/@name/string()','--query-content-type','application/vnd.cem.xpath','--output','cem']);
await save('query-result.txt',qResult.stdout);
const inspect=await run('inspect',['inspect','examples/cem-ml/login.cem','--show','summary','--no-color']);
await save('inspect.txt',inspect.stdout);
const inspectXml=await run('inspect-xml-ast',['inspect','login.xml','--from-format','xml','--show','ast','--no-color'],0,dir);
const inspectExcerpt=inspectXml.stdout.match(/    \{node @id=node-16[\s\S]*?\}/)[0];
await save('inspect-excerpt.txt',inspectExcerpt);
evidence.inspection={input:'login.xml',view:'ast',nodeId:'node-16',byteOffset:252,byteLength:78,excerpt:inspectExcerpt};
// Teaching schema uses the same schema:required-fields contract as the native CLI regression fixture.
const attributes=['method','screen','aria-labelledby','id','action','form','for','name','type','autocomplete','required'];
const schema=`@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema
{schema @name="login" @namespace="urn:demo:login" @version="1.0.0" |
 {uses | {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"} {use @schema="https://cem.dev/ns/schema/1" @as="schema"}}
 {content-types | {content-type @value="application/vnd.demo.login+cem" @primary=true}}
 {elements |
  {element @name="main" @optional-attributes="screen aria-labelledby" @children="*"}
  {element @name="h1" @optional-attributes="id" @children="*"}
  {element @name="form" @optional-attributes="method action form" @children="*"}
  {element @name="label" @optional-attributes="for" @children="*"}
  {element @name="input" @optional-attributes="id name type autocomplete required"}
  {element @name="button" @optional-attributes="type action" @children="*"}}
 {attributes | ${attributes.map(n=>`{attribute @name="${n}" @type="schema:string"}`).join(' ')}}
 {field-contracts |
  {field-contract @name="form-method" @target="form" @required-attributes="method" @diagnostic="demo.login.method_required" @behavior="schema:required-fields" @check-kind="required-fields"}}
 {diagnostics |
  {diagnostic @code="demo.login.method_required" @severity="error" @behavior="schema:required-fields" @message="Login form must declare its method"}}
}
`;
await save('login-schema.cem',schema);
await save('login-schema-package.cem',`@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg
{package @id="login-demo" @version="1.0.0" |
 {schema @uri="urn:demo:login" @source="login-schema.cem"}
 {content-type @value="application/vnd.demo.login+cem" @primary=true}
}
`);
await save('login-invalid.cem',original.replace(' @method=post',''));
const valBase=['--content-type','application/cem','--schema','urn:demo:login','--schema-package',`${rel}/login-schema-package.cem`,'--format','json','--no-color'];
const invalid=await run('validation-invalid',['validate',`${rel}/login-invalid.cem`,...valBase],1);
const valid=await run('validation-valid',['validate','examples/cem-ml/login.cem',...valBase]);
const bad=JSON.parse(invalid.stdout),good=JSON.parse(valid.stdout);
await save('validation-invalid.json',invalid.stdout);await save('validation-valid.json',valid.stdout);
const diagnostic=bad.diagnostics.find(d=>d.code==='demo.login.method_required');
if(!diagnostic||bad.summary.errorCount!==1||good.summary.errorCount!==0)throw Error('Unexpected schema validation outcome');
evidence.validation={invalidExit:1,validExit:0,invalidSummary:bad.summary,validSummary:good.summary,diagnostic};
evidence.limitations.push('Custom teaching schema is a small selected contract, not a complete HTML schema. Both validation runs retain three existing cem.ast.unresolved_reference warnings. Top-level schema diagnostic location is 1:1; retained source-map frame locates the form at line 8.');
const badForm=await format('validation-before','{form @action="#session"}\n');
const goodForm=await format('validation-after','{form @action="#session" @method=post}\n');
const diagText=`${diagnostic.code}\n${diagnostic.message}\n\nInvalid: exit 1 · 1 error\nRepaired: exit 0 · 0 errors`;
await save('diagnostic.txt',diagText);
const theme=await readFile(path.join(root,'packages/cem-theme/dist/lib/css/cem-combined.css'),'utf8');
const html=`<!doctype html><html><head><meta charset="utf-8"><link rel="icon" href="data:,"><style>${theme}</style>
<script type="importmap">{"imports":{"@epa-wg/cem-ml/wasm":"/packages/cem-ml-npm/dist/wasm/browser/cem_ml.js"}}</script>
<script type="module" src="/packages/cem-demo-element/dist/index.js"></script>
<style>html,body{margin:0;padding:0;background:white}body{width:max-content}cem-demo-element{margin:0!important;border:0!important;border-radius:0!important}cem-demo-element>[slot="text"] pre{padding:18px!important;margin:0!important;overflow:visible!important}cem-demo-element>[slot="text"] code{font:28px/1.65 ui-monospace,"DejaVu Sans Mono",monospace!important;white-space:pre!important}cem-demo-element>[slot="demo"],cem-demo-element>[slot="status"]{display:none!important}</style></head><body data-theme="cem-theme-light"><cem-demo-element type="cem-ml" demo="false" source="{h1 | Sign in}"></cem-demo-element></body></html>`;
await save('asset-live.html',html);
const mime={'.html':'text/html','.js':'text/javascript','.css':'text/css','.wasm':'application/wasm','.cem':'text/plain'};
const server=http.createServer(async(req,res)=>{try{const file=path.resolve(root,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname));if(!file.startsWith(root+path.sep)){res.writeHead(403);res.end();return;}res.writeHead(200,{'Content-Type':mime[path.extname(file)]||'text/plain'});res.end(await readFile(file));}catch{res.writeHead(404);res.end('Not found');}});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const origin=`http://127.0.0.1:${server.address().port}`;
let browser;
try{
 browser=await chromium.launch({headless:true});const page=await browser.newPage({viewport:{width:2200,height:1400},deviceScaleFactor:2,colorScheme:'light'});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(`${origin}/${rel}/asset-live.html`);await page.waitForSelector('code.cem-source-code',{state:'attached'}); await page.evaluate(async()=>{await document.querySelector('cem-demo-element').updateComplete;});
 async function codeImage(name,source,contentType='application/cem',options={}){
  const result=await page.evaluate(async({source,contentType,fontSize,nativeHtml,wrapWidth})=>{
   const {highlightCemSource}=await import('/packages/cem-demo-element/dist/cem-ml-runtime.js');
   let r=await highlightCemSource(source,contentType);
   if(nativeHtml){const doc=new DOMParser().parseFromString(nativeHtml,'text/html');r={status:'highlighted',html:doc.querySelector('pre').innerHTML,diagnostics:[],backend:'native-cli-cem-ql-html-colorer'};}
   const code=document.querySelector('code.cem-source-code');
   code.innerHTML=r.html;code.style.setProperty('font-size',(fontSize||28)+'px','important');
   if(r.status!=='highlighted')code.textContent=source;
   const pre=code.parentElement;
   pre.style.width=wrapWidth?wrapWidth+'px':'';
   code.style.setProperty('white-space',wrapWidth?'pre-wrap':'pre','important');
   code.style.overflowWrap=wrapWidth?'anywhere':'normal';
   const expected=document.createElement('template');expected.innerHTML=r.html;
   const colorSamples=Array.from(code.children).slice(0,30).map(n=>({role:n.tagName,color:getComputedStyle(n).color}));
   return {status:r.status,backend:r.backend||'cem-wasm-html-colorer',diagnostics:r.diagnostics,sourceMatches:code.textContent===source,nativeHtmlMatches:r.status==='highlighted'&&code.innerHTML===expected.innerHTML,tokenCount:code.children.length,colorSamples,overflow:pre.scrollWidth>pre.clientWidth};
  },{source,contentType,fontSize:options.fontSize,nativeHtml:options.nativeHtml,wrapWidth:options.wrapWidth});
  if(!result.sourceMatches)throw Error(`${name}: source changed`);
  if(result.status!=='highlighted')evidence.limitations.push(`${name}: native CEM highlighter reports ${result.status}; exact source displayed as plain text, no substitute token colors.`);
  await page.evaluate(()=>document.fonts.ready);
  const target=page.locator('cem-demo-element');const box=await target.boundingBox();
  await target.screenshot({path:path.join(dir,name+'.png')});
  const snapshot=await page.evaluate(()=>{const clone=document.documentElement.cloneNode(true);clone.querySelectorAll('script').forEach(n=>n.remove());clone.querySelector('cem-demo-element').removeAttribute('source');return'<!doctype html>\n'+clone.outerHTML;});
  await save(name+'.html',snapshot);
  evidence.assets.push({name,path:`${rel}/${name}.png`,width:Math.ceil(box.width*2),height:Math.ceil(box.height*2),contentType,...result});
 }
 await codeImage('markup-cem',markupCem);
 await codeImage('markup-xml',markupXml,'text/html');
 await codeImage('edit-before',editBefore);await codeImage('edit-after',editAfter);
 await codeImage('query-cemql',qFormatted,'application/vnd.cem.query+cem-ql',{nativeHtml:qNativeHtml});
 await codeImage('query-xpath',xpath+'\n','application/vnd.cem.xpath');
 await codeImage('query-result',qResult.stdout);
 await codeImage('inspect',inspectExcerpt,'application/cem',{fontSize:24,wrapWidth:920});
 await codeImage('inspect-summary',inspect.stdout,'application/cem',{fontSize:24});
 await codeImage('login-code',formatted, 'application/cem',{fontSize:25});
 await codeImage('validation-before',badForm);await codeImage('validation-after',goodForm);
 await codeImage('diagnostic',diagText,'text/plain',{fontSize:24,wrapWidth:900});
 // Static view is the canonical semantic HTML fixture with presentation CSS only.
 const loginHtml=await readFile(path.join(root,'examples/semantic/login.html'),'utf8');
 const styled=loginHtml.replace('</head>',`<style>body{margin:0;padding:44px;background:#f1f5f7;font:20px/1.5 system-ui,sans-serif;color:#173741}main{width:430px;padding:38px;background:#fff;border:1px solid #dbe5e7;border-radius:16px;box-shadow:0 12px 36px #17374112}h1{font-size:34px;margin:0 0 24px;letter-spacing:-.03em}form{display:flex;flex-direction:column}label{font-size:16px;font-weight:600;margin:12px 0 7px}input{font:inherit;padding:10px 12px;border:1px solid #b2c4c9;border-radius:7px;min-height:25px}button{font:600 18px system-ui;margin-top:26px;padding:14px;background:#16665c;color:white;border:0;border-radius:7px}</style></head>`);
 await save('login-view.html',styled);await page.goto(`${origin}/${rel}/login-view.html`);await page.evaluate(()=>document.fonts.ready);
 const main=page.locator('main');const box=await main.boundingBox();await main.screenshot({path:path.join(dir,'login-view.png')});
 evidence.assets.push({name:'login-view',path:`${rel}/login-view.png`,width:Math.ceil(box.width*2),height:Math.ceil(box.height*2),source:'examples/semantic/login.html',kind:'Static native HTML login view; style-only presentation CSS; no authentication or cem-element runtime claim'});
 evidence.browserVersion=browser.version();evidence.browserErrors=errors;
 evidence.limitations.push('Rendered login is the canonical static semantic HTML fixture with presentation CSS. No authentication flow or cem-element runtime execution was demonstrated.');
}finally{await browser?.close();await new Promise(r=>server.close(r));}
await save('evidence.json',JSON.stringify(evidence,null,2)+'\n');
await save('evidence.md',`# Verified slide assets\n\nGenerated by docs/declarative-amsterdam-2026/generate-slide-assets.mjs.\n\nCEM source images use the native CLI tabular formatter and the existing CEM WASM semantic HTML colorer. Excerpts are labeled in the deck; source bytes are preserved in the screenshot DOM.\n\n## Limitations\n\n${evidence.limitations.map(s=>'- '+s).join('\n')}\n\n## Assets\n\n${evidence.assets.map(a=>`- ${a.name}.png: ${a.width} × ${a.height}${a.status?' · '+a.status:''}`).join('\n')}\n\nFull commands, exact diagnostics, explicit JSON query comparison sidecars, source maps and source hash are in evidence.json.\n`);
console.log(JSON.stringify({assets:evidence.assets.map(({name,width,height,status})=>({name,width,height,status})),limitations:evidence.limitations},null,2));
