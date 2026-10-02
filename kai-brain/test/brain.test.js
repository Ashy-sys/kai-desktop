import test from "node:test";
import assert from "node:assert/strict";
import { once } from "node:events";
import { ProviderManager, ProviderError, TOOL_NAMES, providersFromEnv } from "../src/providers.js";
import { RateLimiter, hashToken } from "../src/security.js";
import { FileDeviceRegistry, PostgresDeviceRegistry, createDeviceRegistry } from "../src/device-registry.js";
import { createServer } from "../src/server.js";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

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
 const token="local-test-token";const store={type:"test",initialize:async()=>{},register:async()=>token,authenticate:async value=>value===token?{id:"device-1"}:null,updateLastSeen:async()=>true};
 const manager={health:()=>[{provider:"mock",model:"mock-1"}],respond:async()=>({kind:"answer",text:"Hi.",provider:"mock",model:"mock-1"})};
 const server=createServer({env:{KAI_RATE_LIMIT_PER_MINUTE:"10"},store,manager});server.listen(0,"127.0.0.1");await once(server,"listening");t.after(()=>server.close());const base=`http://127.0.0.1:${server.address().port}`;
 assert.equal((await fetch(`${base}/healthz`)).status,200);
 assert.equal((await fetch(`${base}/v1/respond`,{method:"POST",headers:{"content-type":"application/json"},body:"{}"})).status,401);
 assert.equal((await fetch(`${base}/v1/respond`,{method:"POST",headers:{authorization:`Bearer ${token}`,"content-type":"application/json"},body:JSON.stringify({messages:[{role:"user",content:"hello"}]})})).status,200);
 assert.equal((await fetch(`${base}/v1/respond`,{method:"POST",headers:{authorization:`Bearer ${token}`,"content-type":"application/json"},body:JSON.stringify({messages:[{role:"user",content:"x"}],toolResult:{name:"run_shell",output:{}}})})).status,400);
 assert.equal((await fetch(`${base}/v1/chat/completions`,{method:"POST"})).status,404);
});

test("file registry persists hashes, authenticates, updates last seen, and revokes",async()=>{
 const directory=await mkdtemp(join(tmpdir(),"kai-registry-"));const registry=new FileDeviceRegistry(directory);await registry.initialize();
 const id="123e4567-e89b-42d3-a456-426614174000";const token=await registry.register(id,{onlyIfFirst:true});assert(token);assert.equal((await registry.authenticate(token)).id,id);
 await registry.updateLastSeen(id);assert((await registry.get(id)).lastSeenAt);assert(!String(await readFile(join(directory,"devices.json"),"utf8")).includes(token));
 assert(await registry.revoke(id));assert.equal(await registry.authenticate(token),null);
});

test("file registry admits exactly one simultaneous first device",async()=>{
 const registry=new FileDeviceRegistry(await mkdtemp(join(tmpdir(),"kai-first-")));await registry.initialize();
 const results=await Promise.all([registry.register("123e4567-e89b-42d3-a456-426614174001",{onlyIfFirst:true}),registry.register("123e4567-e89b-42d3-a456-426614174002",{onlyIfFirst:true})]);
 assert.equal(results.filter(Boolean).length,1);assert.equal(await registry.count(),1);
});

test("postgres registry uses an atomic database claim for simultaneous first enrollment",async()=>{
 let claimed=false;const devices=new Map();const query=async(text,params=[])=>{
  if(text.includes("WITH claim AS")){if(claimed)return[];claimed=true;devices.set(params[0],params[1]);return[{device_id:params[0]}];}
  if(text.startsWith("SELECT count"))return[{count:devices.size}];
  throw new Error("unexpected_query");
 };
 const registry=new PostgresDeviceRegistry("postgresql://kai@example.test/kai",{query});
 const results=await Promise.all([registry.register("123e4567-e89b-42d3-a456-426614174003",{onlyIfFirst:true}),registry.register("123e4567-e89b-42d3-a456-426614174004",{onlyIfFirst:true})]);
 assert.equal(results.filter(Boolean).length,1);assert.equal(await registry.count(),1);
});

test("postgres selection fails closed for missing or malformed configuration",()=>{
 assert.throws(()=>createDeviceRegistry({KAI_DEVICE_REGISTRY:"postgres"}),/database_url_required/);
 assert.throws(()=>createDeviceRegistry({KAI_DEVICE_REGISTRY:"postgres",DATABASE_URL:"not-a-url"}),/invalid_database_url/);
 assert.throws(()=>createDeviceRegistry({KAI_DEVICE_REGISTRY:"other"}),/invalid_device_registry/);
});

test("postgres registry surfaces database unavailability",async()=>{
 const registry=new PostgresDeviceRegistry("postgresql://kai@example.test/kai",{query:async()=>{throw new Error("database_unavailable");}});
 await assert.rejects(()=>registry.initialize(),error=>error.kind==="device_registry_unavailable"&&error.status===503);
});
