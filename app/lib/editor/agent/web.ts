// Provider-neutral replacement for the reference agent's host-specific WebFetch.
// Parse an inert template in the existing network-blocked export runtime, never navigate to it.
type WebAsset={url:string;kind:string;label?:string};
export function agentWebRead(input:{url:string;contentType:string;body:string}){
  if(input.body.length>2*1024*1024)throw new Error('网页内容超过读取限制');
  const absolute=(raw:string,base=input.url)=>{
    if(raw.length>8192)return;
    try{const url=new URL(raw,base);if(url.protocol!=='https:'||url.username||url.password||url.href.length>8192)return;url.hash='';return url.href}catch{return}
  };
  const assets:WebAsset[]=[],links:{url:string;text:string}[]=[],styles:string[]=[],assetKeys=new Set<string>(),linkKeys=new Set<string>();
  let assetCount=0,linkCount=0;
  const addAsset=(raw:string,kind:string,label?:string,base?:string)=>{
    const url=absolute(raw,base);if(!url||assetKeys.has(url))return;assetKeys.add(url);assetCount++;
    if(assets.length<2000)assets.push({url,kind,...(label?{label:label.slice(0,240)}:{})});
  };
  const css=(text:string,base=input.url)=>{
    for(const match of text.matchAll(/url\(\s*(?:"([^"]+)"|'([^']+)'|([^\s)]+))\s*\)/gi)){
      const raw=match[1]??match[2]??match[3];addAsset(raw,/\.(woff2?|ttf|otf)(?:[?#]|$)/i.test(raw)?'font':'css-asset',undefined,base);
    }
    for(const match of text.matchAll(/@import\s+["']([^"']+)["']/gi))addAsset(match[1],'stylesheet',undefined,base);
  };
  if(input.contentType==='text/css'){
    css(input.body);return {url:input.url,title:'',text:input.body,assets,assetCount,links,linkCount,styles:[],kind:'stylesheet',scripted:false};
  }
  if(input.contentType!=='text/html'&&input.contentType!=='application/xhtml+xml')return {url:input.url,title:'',text:input.body,assets,assetCount,links,linkCount,styles:[],kind:'text',scripted:false};
  const template=document.createElement('template');template.innerHTML=input.body;
  const doc=template.content;
  const base=absolute(doc.querySelector('base[href]')?.getAttribute('href')??'')??input.url;
  const title=doc.querySelector('title')?.textContent?.trim().slice(0,500)??'';
  const description=doc.querySelector('meta[name="description"],meta[property="og:description"]')?.getAttribute('content')?.slice(0,2000)??'';
  const canonical=absolute(doc.querySelector('link[rel="canonical"]')?.getAttribute('href')??'',base);
  for(const el of doc.querySelectorAll('img')){
    const label=el.getAttribute('alt')??undefined;
    for(const name of ['src','data-src','data-original']){const src=el.getAttribute(name);if(src)addAsset(src,'image',label,base);}
    for(const candidate of (el.getAttribute('srcset')??'').split(',')){const raw=candidate.trim().split(/\s+/)[0];if(raw)addAsset(raw,'image',label,base);}
  }
  for(const el of doc.querySelectorAll('video,audio,source')){
    const raw=el.getAttribute('src');if(raw)addAsset(raw,el.tagName.toLowerCase()==='audio'?'audio':'media',undefined,base);
    const poster=el.getAttribute('poster');if(poster)addAsset(poster,'image',undefined,base);
  }
  for(const el of doc.querySelectorAll('meta[property="og:image"],meta[name="twitter:image"]')){const raw=el.getAttribute('content');if(raw)addAsset(raw,'image',el.getAttribute('property')??'twitter:image',base);}
  for(const el of doc.querySelectorAll('link[href]')){
    const rel=el.getAttribute('rel')??'',as=el.getAttribute('as');
    if(/stylesheet|icon/i.test(rel)||as==='font')addAsset(el.getAttribute('href')!,as==='font'?'font':/stylesheet/i.test(rel)?'stylesheet':'icon',undefined,base);
  }
  for(const el of doc.querySelectorAll('a[href]')){
    const raw=el.getAttribute('href')!,url=absolute(raw,base);if(!url||linkKeys.has(url))continue;linkKeys.add(url);linkCount++;
    const text=(el.textContent??'').replace(/\s+/g,' ').trim().slice(0,240);
    if(links.length<2000)links.push({url,text});
    if(/\.(svg|png|jpe?g|gif|webp|mp4|mov|webm|mp3|wav|flac|ogg|woff2?|ttf|otf)(?:[?#]|$)/i.test(url))addAsset(url,/\.(woff2?|ttf|otf)(?:[?#]|$)/i.test(url)?'font':'file',text);
  }
  for(const el of doc.querySelectorAll('style')){const text=el.textContent??'';css(text,base);if(styles.length<12)styles.push(text.slice(0,4000));}
  for(const el of doc.querySelectorAll('[style]'))css(el.getAttribute('style')??'',base);
  const palette=[...new Set((styles.join('\n').match(/#[0-9a-fA-F]{3,8}\b/g)??[]))].slice(0,64);
  const fontFamilies=[...new Set(styles.join('\n').match(/font-family\s*:\s*[^;}]+/gi)??[])].slice(0,64);
  for(const el of doc.querySelectorAll('script,style,noscript,template,iframe,object,embed,svg,form,[hidden],[aria-hidden="true"]'))el.remove();
  const blocks=new Set(['P','DIV','SECTION','ARTICLE','MAIN','HEADER','FOOTER','NAV','LI','H1','H2','H3','H4','H5','H6','TR','BR','HR','BLOCKQUOTE','PRE']);
  // Iterative traversal avoids stack exhaustion on a deeply nested page.
  const chunks:string[]=[],stack:{node:Node;exit:boolean}[]=Array.from(doc.childNodes).reverse().map(node=>({node,exit:false}));
  while(stack.length){const {node,exit}=stack.pop()!;
    if(node.nodeType===Node.TEXT_NODE){chunks.push(node.textContent??'');continue;}
    if(node.nodeType!==Node.ELEMENT_NODE)continue;
    const el=node as Element;if(blocks.has(el.tagName))chunks.push('\n');if(exit)continue;
    stack.push({node,exit:true});for(let i=el.childNodes.length-1;i>=0;i--)stack.push({node:el.childNodes[i],exit:false});
  }
  const text=chunks.join('').replace(/[\t\r ]+/g,' ').replace(/ *\n */g,'\n').replace(/\n{3,}/g,'\n\n').trim();
  return {url:input.url,title,description,canonical,text,assets,assetCount,links,linkCount,styles,palette,fontFamilies,kind:'page',scripted:false};
}
