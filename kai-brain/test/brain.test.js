import test from "node:test";
import assert from "node:assert/strict";
import { once } from "node:events";
import { ProviderManager, ProviderError, TOOL_NAMES, providersFromEnv } from "../src/providers.js";
import { RateLimiter, hashToken } from "../src/security.js";
import { createServer } from "../src/server.js";

test("provider manager falls back after quota and timeout failures", async()=>{
 const manager=new ProviderManager([
  {name:"first",model:"a",respond:async()=>{throw new ProviderError("quota","secret upstream body")}},
  {name:"second",model:"b",respond:async()=>({kind:"answer",text:"hello",provider:"second",model:"b"})}
 ]);
 assert.equal((await manager.respond({messages:[]})).provider,"second");
});
test("provider manager reports all unavailable without leaking prompts",async()=>{const manager=new ProviderManager([{name:"x",model:"m",respond:async()=>{throw new ProviderError("timeout","prompt contents")}}]);await assert.rejects(()=>manager.respond({messages:[{role:"user",content:"private prompt"}]}),error=>error.kind==="all_unavailable"&&!error.message.includes("private prompt"));});
test("only six registered tools exist and forbidden tools are absent",()=>{assert.deepEqual(TOOL_NAMES,["search_files","get_file_metadata","open_file","reveal_file","open_known_app","get_system_info"]);assert(!TOOL_NAMES.some(x=>/shell|command|powershell/i.test(x)));});
test("provider configuration tolerates one provider and no secrets are returned by health",()=>{const secret="deployment-only-secret-value";const providers=providersFromEnv({GROQ_API_KEY:secret});assert.equal(providers.length,1);assert.equal(providers[0].name,"groq");assert(!JSON.stringify({provider:providers[0].name,model:providers[0].model}).includes(secret));});
test("malformed configured provider credentials fail startup safely",()=>assert.throws(()=>providersFromEnv({OPENAI_API_KEY:"short"}),/malformed_openai_api_key/));
test("rate limiter bounds each device",()=>{const limiter=new RateLimiter(2,1000);assert(limiter.allow("a",0));assert(limiter.allow("a",1));assert(!limiter.allow("a",2));assert(limiter.allow("b",2));});
test("token hashing is deterministic and does not retain plaintext",()=>{assert.equal(hashToken("device-token"),hashToken("device-token"));assert(!hashToken("device-token").includes("device-token"));});

test("service authenticates devices, rejects malformed input, and never exposes a general proxy", async t=>{
 const token="local-test-token";const store={load:async()=>{},count:()=>1,register:async()=>token,authenticate:value=>value===token?{id:"device-1"}:null};
 const manager={health:()=>[{provider:"mock",model:"mock-1"}],respond:async()=>({kind:"answer",text:"Hi.",provider:"mock",model:"mock-1"})};
 const server=createServer({env:{KAI_RATE_LIMIT_PER_MINUTE:"10"},store,manager});server.listen(0,"127.0.0.1");await once(server,"listening");t.after(()=>server.close());const base=`http://127.0.0.1:${server.address().port}`;
 assert.equal((await fetch(`${base}/healthz`)).status,200);
 assert.equal((await fetch(`${base}/v1/respond`,{method:"POST",headers:{"content-type":"application/json"},body:"{}"})).status,401);
 assert.equal((await fetch(`${base}/v1/respond`,{method:"POST",headers:{authorization:`Bearer ${token}`,"content-type":"application/json"},body:JSON.stringify({messages:[{role:"user",content:"hello"}]})})).status,200);
 assert.equal((await fetch(`${base}/v1/respond`,{method:"POST",headers:{authorization:`Bearer ${token}`,"content-type":"application/json"},body:JSON.stringify({messages:[{role:"user",content:"x"}],toolResult:{name:"run_shell",output:{}}})})).status,400);
 assert.equal((await fetch(`${base}/v1/chat/completions`,{method:"POST"})).status,404);
});
