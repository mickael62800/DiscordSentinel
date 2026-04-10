# security-bot

**Rôle** : Détecte les raids, comptes alts, spams de join massifs et applique quarantaine, CAPTCHA, slowmode et lockdown automatiques.

## Commandes / Events Discord principaux

- Slash `/security status` — état des défenses (raid detector, quarantine, slowmode, lockdown)
- Slash `/security history` — derniers événements de sécurité (N=5 par défaut)
- Event `guild_member_addition` — vérification âge compte, détection raid, captcha
- Event `message` — slowmode adaptatif si raid détecté

## Dépendances externes

- API interne (`security_events`, `manual_watched_users`)
- Discord Gateway + REST
- Service CAPTCHA (optionnel)

## Modules clés

- `src/security/raid_detector.rs` — suivi des joins récents et seuil d'alerte
- `src/security/account_checker.rs` — vérification de l'âge minimum des comptes
- `src/security/alt_detector.rs` — clustering par date de création et similarité de nom
- `src/security/quarantine.rs` / `lockdown.rs` / `slowmode.rs` — actions réactives

## Variables d'env

- `SECURITY_DISCORD_TOKEN`
- `API_BASE_URL`
- `API_KEY`
- `RAID_JOIN_THRESHOLD` / `RAID_JOIN_WINDOW_SECS`
- `CAPTCHA_ENABLED`
- `QUARANTINE_ENABLED`

## Cache Serenity (Phase 1)

**Tier : `medium`** — cache messages récents pour contexte.

## Note Phase 2

Les events sont stockés dans `security_events.user_ids` en JSONB avec un index GIN (migration 100) → les queries du style `WHERE user_ids @> '["<user_id>"]'::jsonb` sont accélérées 10-50×.
