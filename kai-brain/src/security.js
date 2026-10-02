import { createHash, randomBytes, timingSafeEqual } from "node:crypto";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

export const hashToken = token => createHash("sha256").update(token).digest("hex");
export const newToken = () => randomBytes(32).toString("base64url");
export const safeEqual = (a, b) => {
  const left = Buffer.from(String(a)); const right = Buffer.from(String(b));
  return left.length === right.length && timingSafeEqual(left, right);
};

export class DeviceStore {
  constructor(dataDir) { this.path = join(dataDir, "devices.json"); this.devices = new Map(); }
  async load() {
    try { const data = JSON.parse(await readFile(this.path, "utf8")); this.devices = new Map(data.devices.map(d => [d.id, d])); }
    catch (error) { if (error.code !== "ENOENT") throw error; }
  }
  count() { return this.devices.size; }
  authenticate(token) {
    if (!token) return null; const wanted = hashToken(token);
    return [...this.devices.values()].find(device => !device.revokedAt && safeEqual(device.tokenHash, wanted)) ?? null;
  }
  async register(id) {
    if (!/^[a-f0-9-]{36}$/i.test(id)) throw new Error("invalid_device_id");
    const token = newToken(); this.devices.set(id, { id, tokenHash:hashToken(token), createdAt:new Date().toISOString(), revokedAt:null });
    await this.save(); return token;
  }
  async revoke(id) { const device=this.devices.get(id); if (!device) return false; device.revokedAt=new Date().toISOString(); await this.save(); return true; }
  async save() {
    await mkdir(dirname(this.path), { recursive:true }); const temp=`${this.path}.tmp`;
    await writeFile(temp, JSON.stringify({devices:[...this.devices.values()]}, null, 2), {mode:0o600}); await rename(temp, this.path);
  }
}

export class RateLimiter {
  constructor(limit=12, windowMs=60_000) { this.limit=limit; this.windowMs=windowMs; this.buckets=new Map(); }
  allow(key, now=Date.now()) { const recent=(this.buckets.get(key)??[]).filter(t=>now-t<this.windowMs); if(recent.length>=this.limit){this.buckets.set(key,recent);return false;}recent.push(now);this.buckets.set(key,recent);return true; }
}
