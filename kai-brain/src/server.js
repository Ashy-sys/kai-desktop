import http from "node:http";
import { fileURLToPath } from "node:url";
import { DeviceStore, RateLimiter, safeEqual } from "./security.js";
import { ProviderManager, providersFromEnv, TOOL_NAMES } from "./providers.js";

const MAX_BODY=64*1024; const json=(res,status,body)=>{res.writeHead(status,{"content-type":"application/json","cache-control":"no-store","x-content-type-options":"nosniff"});res.end(JSON.stringify(body));};
const readJson=async req=>{let size=0;const chunks=[];for await(const chunk of req){size+=chunk.length;if(size>MAX_BODY)throw new Error("body_too_large");chunks.push(chunk)}return JSON.parse(Buffer.concat(chunks).toString("utf8"));};
const validRequest=body=>body&&Array.isArray(body.messages)&&body.messages.length<=16&&body.messages.every(m=>["user","assistant"].includes(m.role)&&typeof m.content==="string"&&m.content.length<=4000)&&(!body.toolResult||(TOOL_NAMES.includes(body.toolResult.name)&&typeof body.toolResult.output==="object"));
export function createServer({env=process.env,store,manager,now=()=>Date.now()}={}){
 const devices=store??new DeviceStore(env.KAI_BRAIN_DATA_DIR??"./data");const providers=manager??new ProviderManager(providersFromEnv(env));const deviceLimit=new RateLimiter(Number(env.KAI_RATE_LIMIT_PER_MINUTE??12));const ipLimit=new RateLimiter(30);
 const server=http.createServer(async(req,res)=>{const requestId=crypto.randomUUID();const ip=req.socket.remoteAddress??"unknown";try{
  if(req.method==="GET"&&req.url==="/healthz")return json(res,200,{ok:true,providers:providers.health().map(p=>({provider:p.provider,model:p.model}))});
  if(req.method==="POST"&&req.url==="/v1/devices/register"){if(!ipLimit.allow(`register:${ip}`,now()))return json(res,429,{error:"rate_limited",requestId});const body=await readJson(req);const registrationCode=req.headers["x-kai-registration-code"]??"";const firstAllowed=env.KAI_ALLOW_FIRST_DEVICE_REGISTRATION==="true"&&devices.count()===0;const codeAllowed=env.KAI_REGISTRATION_CODE&&safeEqual(registrationCode,env.KAI_REGISTRATION_CODE);if(!firstAllowed&&!codeAllowed)return json(res,403,{error:"registration_closed",requestId});const token=await devices.register(body.deviceId);console.info(JSON.stringify({event:"device_registered",requestId,deviceId:body.deviceId}));return json(res,201,{deviceToken:token});}
  if(req.method==="POST"&&req.url==="/v1/respond"){const auth=String(req.headers.authorization??"").replace(/^Bearer\s+/i,"");const device=devices.authenticate(auth);if(!device)return json(res,401,{error:"unauthorized",requestId});if(!deviceLimit.allow(device.id,now())||!ipLimit.allow(`respond:${ip}`,now()))return json(res,429,{error:"rate_limited",requestId});const body=await readJson(req);if(!validRequest(body))return json(res,400,{error:"invalid_request",requestId});const result=await providers.respond(body);console.info(JSON.stringify({event:"response_completed",requestId,deviceId:device.id,provider:result.provider,model:result.model,kind:result.kind}));return json(res,200,{...result,requestId});}
  json(res,404,{error:"not_found",requestId});
 }catch(error){console.error(JSON.stringify({event:"request_failed",requestId,kind:error.kind??error.message??"internal"}));json(res,error.status??400,{error:error.kind??"invalid_request",requestId});}});
 server.initialize=()=>devices.load();return server;
}
if(process.argv[1]===fileURLToPath(import.meta.url)){const server=createServer();await server.initialize();const port=Number(process.env.KAI_BRAIN_PORT??8787);server.listen(port,process.env.KAI_BRAIN_HOST??"127.0.0.1",()=>console.info(JSON.stringify({event:"brain_started",port})));const shutdown=()=>server.close(()=>process.exit(0));process.on("SIGTERM",shutdown);process.on("SIGINT",shutdown);}
