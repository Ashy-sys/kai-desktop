import { createHash, randomBytes, timingSafeEqual } from "node:crypto";

export const hashToken = token => createHash("sha256").update(token).digest("hex");
export const newToken = () => randomBytes(32).toString("base64url");
export const safeEqual = (a, b) => {
  const left = Buffer.from(String(a)); const right = Buffer.from(String(b));
  return left.length === right.length && timingSafeEqual(left, right);
};

export class RateLimiter {
  constructor(limit=12, windowMs=60_000) { this.limit=limit; this.windowMs=windowMs; this.buckets=new Map(); }
  allow(key, now=Date.now()) { const recent=(this.buckets.get(key)??[]).filter(t=>now-t<this.windowMs); if(recent.length>=this.limit){this.buckets.set(key,recent);return false;}recent.push(now);this.buckets.set(key,recent);return true; }
}
