# Chat ontwerp (container-per-user)

## Kernkeuze
- Elke user krijgt bij inloggen een **eigen (lichte) Docker container**.
- Vanuit die container is er **directe verbinding** met de `chat-api` (dus **niet** via de website als proxy).
- De website blijft de plek waar de user inlogt en waar we auth cookies/session afhandelen, maar de container kan zelfstandig tokens uitwisselen met de auth-service.

## Authenticatie flow (aanrader)
1. User logt in op de website → auth-service geeft tokens terug (bijv. access + refresh).
2. De per-user container bewaart tokens **server-side** (nooit in browser-localStorage).
3. Calls naar chat-api:
   - REST: `Authorization: Bearer <access_jwt>`
   - WebSocket: gebruik `ws-ticket` (kortlevend) i.p.v. `?token=` in de URL.

## Waarom dit goed is
- Geen “open” chat-api: chat-api valideert elk request met JWT + permission checks (`activity.can_chat()`).
- Tokens blijven server-side in de container → veilig en simpel (geen CORS/cookie complexiteit in de browser).
- Schaalbaar: containers zijn licht, en chat-api blijft één centrale service.

## Minimale eisen aan chat-api
- JWT validatie (JWKS) op REST endpoints.
- `POST /api/v1/ws-ticket` + WS connect met `?ticket=...`.
- Permissions: fail-closed via activitydb (`activity.can_chat(user_id, conversation_id, bit)`).

## Lokale chat cache (offline)
- Gebruik een **aparte** SQLite database in de per-user container: `chat_cache.db`.
- Schema staat in `chat_cache/schema.sql`.
- Cache policy (advies):
  - per conversatie: max ~300 berichten
  - max ~50 conversaties (LRU op `conversations.updated_at`)
  - TTL ~30 dagen en/of prune naar max
  - wipe op logout/reset
