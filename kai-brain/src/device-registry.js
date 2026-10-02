import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { neon } from "@neondatabase/serverless";
import { hashToken, newToken, safeEqual } from "./security.js";

const DEVICE_ID = /^[a-f0-9]{8}-[a-f0-9]{4}-[1-5][a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/i;
const assertDeviceId = id => { if (!DEVICE_ID.test(id)) throw new Error("invalid_device_id"); };

export class DeviceRegistry {
  async initialize() {}
  async get() { throw new Error("not_implemented"); }
  async register() { throw new Error("not_implemented"); }
  async revoke() { throw new Error("not_implemented"); }
  async count() { throw new Error("not_implemented"); }
  async updateLastSeen() { throw new Error("not_implemented"); }
  async authenticate() { throw new Error("not_implemented"); }
}

export class FileDeviceRegistry extends DeviceRegistry {
  constructor(dataDir) { super(); this.type="file"; this.path=join(dataDir,"devices.json"); this.devices=new Map(); this.queue=Promise.resolve(); }
  async initialize() {
    try { const data=JSON.parse(await readFile(this.path,"utf8")); this.devices=new Map(data.devices.map(d=>[d.id,d])); }
    catch(error) { if(error.code!=="ENOENT") throw error; }
  }
  async get(id) { return this.devices.get(id)??null; }
  async count() { return this.devices.size; }
  async authenticate(token) {
    if(!token) return null; const wanted=hashToken(token);
    return [...this.devices.values()].find(device=>!device.revokedAt&&safeEqual(device.tokenHash,wanted))??null;
  }
  async register(id,{onlyIfFirst=false}={}) {
    assertDeviceId(id);
    return this.exclusive(async()=>{
      if(onlyIfFirst&&this.devices.size!==0)return null;
      const token=newToken(); this.devices.set(id,{id,tokenHash:hashToken(token),createdAt:new Date().toISOString(),lastSeenAt:null,revokedAt:null});
      await this.save(); return token;
    });
  }
  async revoke(id) { return this.exclusive(async()=>{const device=this.devices.get(id);if(!device)return false;device.revokedAt=new Date().toISOString();await this.save();return true;}); }
  async updateLastSeen(id) { return this.exclusive(async()=>{const device=this.devices.get(id);if(!device)return false;device.lastSeenAt=new Date().toISOString();await this.save();return true;}); }
  async exclusive(operation) { const next=this.queue.then(operation,operation);this.queue=next.catch(()=>{});return next; }
  async save() { await mkdir(dirname(this.path),{recursive:true});const temp=`${this.path}.tmp`;await writeFile(temp,JSON.stringify({devices:[...this.devices.values()]},null,2),{mode:0o600});await rename(temp,this.path); }
}

const rows = result => Array.isArray(result)?result:(result?.rows??[]);
export class PostgresDeviceRegistry extends DeviceRegistry {
  constructor(databaseUrl,{query}={}) {
    super(); validateDatabaseUrl(databaseUrl); this.type="postgres";
    const sql=query?null:neon(databaseUrl);const execute=query??((text,params=[])=>sql.query(text,params));
    this.query=async(text,params=[])=>{try{return await execute(text,params);}catch(cause){const error=new Error("device_registry_unavailable",{cause});error.kind="device_registry_unavailable";error.status=503;throw error;}};
  }
  async initialize() { await this.query("SELECT 1 FROM kai_registry_state LIMIT 1"); }
  async get(id) { const result=await this.query("SELECT device_id AS id, created_at, last_seen_at, revoked_at FROM kai_devices WHERE device_id = $1",[id]);return rows(result)[0]??null; }
  async count() { const result=await this.query("SELECT count(*)::int AS count FROM kai_devices");return Number(rows(result)[0]?.count??0); }
  async authenticate(token) {
    if(!token)return null;const result=await this.query("SELECT device_id AS id, created_at, last_seen_at, revoked_at FROM kai_devices WHERE token_hash = $1 AND revoked_at IS NULL",[hashToken(token)]);return rows(result)[0]??null;
  }
  async register(id,{onlyIfFirst=false}={}) {
    assertDeviceId(id);const token=newToken();const tokenHash=hashToken(token);
    const result=onlyIfFirst
      ? await this.query(`WITH claim AS (
          UPDATE kai_registry_state SET first_device_registered = true
          WHERE singleton = true AND first_device_registered = false RETURNING 1
        ), inserted AS (
          INSERT INTO kai_devices (device_id, token_hash) SELECT $1, $2 FROM claim
          ON CONFLICT (device_id) DO NOTHING RETURNING device_id
        ) SELECT device_id FROM inserted`,[id,tokenHash])
      : await this.query("INSERT INTO kai_devices (device_id, token_hash) VALUES ($1, $2) ON CONFLICT (device_id) DO NOTHING RETURNING device_id",[id,tokenHash]);
    return rows(result).length===1?token:null;
  }
  async revoke(id) { const result=await this.query("UPDATE kai_devices SET revoked_at = now() WHERE device_id = $1 AND revoked_at IS NULL RETURNING device_id",[id]);return rows(result).length===1; }
  async updateLastSeen(id) { const result=await this.query("UPDATE kai_devices SET last_seen_at = now() WHERE device_id = $1 AND revoked_at IS NULL RETURNING device_id",[id]);return rows(result).length===1; }
}

export function validateDatabaseUrl(value) {
  let parsed;try{parsed=new URL(value);}catch{throw new Error("invalid_database_url");}
  if(!["postgres:","postgresql:"].includes(parsed.protocol)||!parsed.hostname||!parsed.username)throw new Error("invalid_database_url");
}

export function createDeviceRegistry(env=process.env,options={}) {
  const type=(env.KAI_DEVICE_REGISTRY??"file").toLowerCase();
  if(type==="file")return new FileDeviceRegistry(env.KAI_BRAIN_DATA_DIR??"./data");
  if(type==="postgres"){if(!env.DATABASE_URL)throw new Error("database_url_required");return new PostgresDeviceRegistry(env.DATABASE_URL,options);}
  throw new Error("invalid_device_registry");
}
