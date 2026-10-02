# Kai Brain

Kai Brain is a small Kai-specific HTTPS service. It is not an OpenAI-compatible proxy and exposes only `GET /healthz`, `POST /v1/devices/register`, and authenticated `POST /v1/respond`.

## Trust model

Provider keys exist only in server environment/secrets management. A desktop installation generates a random device ID and receives an opaque 256-bit device token. Kai stores that token in Windows Credential Manager. The server stores only its SHA-256 hash. Tokens can be revoked in the selected registry (an administration CLI/API is deliberately not exposed publicly in V1.2).

For the first personal deployment, set `KAI_ALLOW_FIRST_DEVICE_REGISTRATION=true` on a fresh persistent data volume. Exactly the first device may enroll without a shared secret; registration closes once it succeeds. Set it back to `false` afterward. Additional devices require a deployment-time `KAI_REGISTRATION_CODE`, entered only during enrollment and never shipped in the installer. This is zero-config for the first personal installation, not a mass-distribution identity system. A public multi-user release still needs a proper account/attestation enrollment service.

Rate limits apply per device and source IP. Registration is separately IP-limited. Rate-limit counters are intentionally in memory and reset whenever an instance restarts or traffic moves to another stateless instance. Request bodies are capped at 64 KiB, context at 16 messages, individual messages at 4,000 characters, and output at roughly 500 tokens. Logs contain request IDs, device IDs, provider/model, result kind, and error category—not prompts, tool results, bearer tokens, or provider response bodies.

## Run and deploy

1. Install Node.js 22 or newer.
2. Copy `.env.example` to a secret environment managed by the host. Do not commit `.env`.
3. Choose the registry explicitly:
   - Local development: `KAI_DEVICE_REGISTRY=file` and a stable `KAI_BRAIN_DATA_DIR` (file is also the default when the variable is omitted).
   - Stateless hosting: `KAI_DEVICE_REGISTRY=postgres`, a pooled `DATABASE_URL` for runtime, and a direct/unpooled `DATABASE_URL_UNPOOLED` for migrations.
4. For PostgreSQL, run `npm run db:migrate` once before starting or releasing the service. The migration is idempotent and creates only `kai_devices` plus a one-row enrollment coordination table. It never stores raw tokens, prompts, tool results, provider payloads, or credentials.
5. Configure at least one provider key, then run `npm test`, `npm run check`, and `npm start`.
6. Put the service behind a production HTTPS reverse proxy/load balancer. Bind directly to `127.0.0.1` unless the hosting platform supplies the secure ingress.
7. Set desktop build variable `KAI_BRAIN_URL=https://brain.example.com` before `npm run tauri build`.

PostgreSQL startup fails closed if `DATABASE_URL` is missing, malformed, unreachable, or the migration has not run. The first-device claim and device insert are one atomic SQL statement guarded by the coordination row, so concurrent fresh-deployment enrollment attempts produce exactly one successful device. `GET /healthz` reports the selected registry type and configured provider count, but no connection strings or secrets.

Provider order is Gemini → Groq → OpenAI for whichever credentials are configured. Defaults are stable `gemini-2.5-flash`, Groq production `openai/gpt-oss-20b`, and cost-sensitive `gpt-5.6-luna`. Free/provider quotas are not guaranteed; inspect the provider consoles for the deployment's current limits. When every provider is unavailable, the service returns a temporary-unavailable error and the desktop stays fully functional locally.

Production requirements not automated here: provision an HTTPS hostname, PostgreSQL database, provider secrets, monitoring, database backups, and the first-device enrollment window. Apply the migration with the direct URL, run the service with the pooled URL, confirm `/healthz`, temporarily enable first-device enrollment, enroll exactly one desktop, and disable the enrollment flag. No deployment is performed by this repository.
