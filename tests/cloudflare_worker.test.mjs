import test from 'node:test';
import assert from 'node:assert/strict';
import worker from '../deployment/cloudflare/worker.mjs';

const PUBLIC='https://animus-api.intelpol.workers.dev';
const ORIGIN='https://hidden-origin.example.org';
async function runRequest(path, init={}) {
 const origFetch=globalThis.fetch;
 let upstream;
 globalThis.fetch=async (request) => {
  upstream=request;
  return new Response(null,{status:302,headers:{location:ORIGIN+'/account/', 'x-powered-by':'php', 'set-cookie':'a=b; Domain=hidden-origin.example.org; Path=/; Secure'}});
 };
 try {
  const response=await worker.fetch(new Request(PUBLIC+path,init), {UPSTREAM_ORIGIN:ORIGIN});
  return {response,upstream};
 } finally {globalThis.fetch=origFetch;}
}
test('public origin forwards exact path and rewrite private-origin redirects',async()=>{
 const {response,upstream}=await runRequest('/api/patches/7/download-token?q=1');
 assert.equal(upstream.url,ORIGIN+'/api/patches/7/download-token?q=1');
 assert.equal(response.headers.get('location'),PUBLIC+'/account/');
 assert.ok(!response.headers.get('set-cookie').toLowerCase().includes('domain='));
 assert.equal(response.headers.get('x-powered-by'),null);
});
test('unconfigured upstream fails closed without exposing upstream',async()=>{
 const response=await worker.fetch(new Request(PUBLIC+'/health'),{});
 assert.equal(response.status,503);
});
test('other host is refused',async()=>{
 const response=await worker.fetch(new Request('https://other.example.org/api'),{UPSTREAM_ORIGIN:ORIGIN});
 assert.equal(response.status,421);
});
test('reject any self-referencing upstream',async()=>{
 const response=await worker.fetch(new Request(PUBLIC+'/api'),{UPSTREAM_ORIGIN:PUBLIC});
 assert.equal(response.status,503);
});
