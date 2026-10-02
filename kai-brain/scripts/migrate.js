import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { neon } from "@neondatabase/serverless";
import { validateDatabaseUrl } from "../src/device-registry.js";

const url=process.env.DATABASE_URL_UNPOOLED;
if(!url)throw new Error("DATABASE_URL_UNPOOLED is required for migrations");
validateDatabaseUrl(url);
const sql=neon(url);
const source=await readFile(fileURLToPath(new URL("../migrations/001_device_registry.sql",import.meta.url)),"utf8");
const statements=source.split(/;\s*(?:\r?\n|$)/).map(value=>value.trim()).filter(Boolean);
for(const statement of statements)await sql.query(statement);
console.info(JSON.stringify({event:"migration_complete",migration:"001_device_registry"}));
