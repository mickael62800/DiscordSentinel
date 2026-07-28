# Paramètres à rendre configurables (audit)

> Inventaire des valeurs codées en dur qui devraient être éditables par serveur
> (pattern `config_schema` + `bot_guild_config`), issu d'un audit read-only des
> 4 grappes de modules. Rien n'est modifié ; ce doc sert à trancher clé par clé.
>
> Date : 2026-07-01 · Convention : `clé` (type, défaut = valeur actuelle) — fichier:ligne.

---

## 0. PRIORITÉ ABSOLUE — bugs de câblage (clés existantes IGNORÉES)

Ces réglages **existent déjà dans le dashboard** mais le code ne les lit pas : ils
sont silencieusement sans effet. À corriger **avant** d'ajouter quoi que ce soit —
c'est ce qui trompe le plus l'utilisateur.

| Zone | Symptôme | Fichier:ligne | Correction |
|---|---|---|---|
| **Game Portal** | Les migrations 189/216 déclarent tout un schéma worker (`health_check_interval_secs`, `idle_shutdown_check_interval_secs`, `reconciler_interval_secs`, `rcon_timeout_secs`, `max_auto_restart_attempts`, `auto_restart_on_crash`) — **le code ne lit AUCUNE de ces clés** | `sentinel-worker/domains/game_portal/jobs.rs:52-73`, `server.rs:103`, `worker_jobs.rs:311` | Threader la config dans `jobs::start()` + lire les clés. **Bonus bug** : défaut schéma `max_auto_restart_attempts=3` mais code = `5` |
| **Automod** | ~~Le mute IA + le mute vision hardcodent **600s**, ignorent `mute_duration_secs` existant~~ **✅ RÉSOLU** : les deux chemins lisent désormais `mute_duration_secs` (défaut `DEFAULT_MUTE_DURATION_SECS`) | `message_handler.rs:62`, `review.rs:511`, `vote/finalize.rs:125` | ~~Lire `mute_duration_secs`~~ (fait) |
| **Sécurité** | Le lockdown et le slowmode **persistés** ignorent `lockdown_duration_secs` / la durée configurée (hardcode 600 / 300) | `detectors/lockdown.rs:141`, `detectors/slowmode.rs:80` | Lire la durée configurée |
| **Confessions** | Couleur d'embed hardcodée `0xff5e5e` alors que `default_embed_color_hex` existe ; les modales submit/reply ignorent `min_chars`/`max_chars` | `confessions/mod.rs:503,669,152,378` | Lire les clés existantes |
| **Voice** | Le message anti-flood affiche "30 secondes" en dur au lieu de `voice_flood_mute_duration_secs` | `voice/handlers/message.rs:70,88` | Lire la clé |
| **Worker** | ~~`tamagotchi_tick_interval_secs` et `age_unban_interval` absents de `apply_db_config`~~ **✅ RÉSOLU/OBSOLÈTE** : `age_unban_interval_secs` est surchargeable par DB ; le domaine tamagotchi a été supprimé du worker (jeux retirés) | `sentinel-worker/config.rs` | (fait) |

---

## 1. Nouveaux réglages à exposer (par module, priorisés)

### Modération (`moderation-bot`)
- **HIGH** — poids de score par flag `score_weight_<spam|insult|link|phishing|nsfw|illicit|anger|rage|threat|harassment>` (number, ×10) — `scoring_service.rs:6-17`
- **HIGH** — seuils d'action `score_threshold_<warn|delete|mute|ban>` (number, 2/4/6/9) — `scoring_service.rs:20-23`
- **HIGH** — `strike_window_secs` (3600, fenêtre d'accumulation des strikes) — `strikes.rs:31`
- **HIGH** — `score_mute_duration_secs` (600) — `scoring_service.rs:26`
- **MED** — `risky_recent_account_days` (7) · `appeal_cooldown_secs` (300) · `sanction_remind_before_secs` (3600)
- **LOW** — `review_default_mute_duration_secs`, `escalation_default_duration_secs`, `history_cache_ttl_secs`

*(Note : `scoring_service.rs` est PARTAGÉ moderation+automod, avec des copies inline dans `analyze_message_service.rs:616` et `analyze_image_service.rs:301` — si on expose, toutes doivent lire la config.)*

### Automod (`automod-bot`)
- **MED** — `text_inference_timeout_secs` (5) · `vision_result_timeout_secs` (30) / `vision_result_poll_secs` (1) · `night_mode_strictness_divisor` (2) · seuils/poids par défaut (mêmes constantes que modération)
- **LOW** — `vote_thread_archive_hours` (3j) · `suspicious_files_use_builtin_list` (bool)

### Sécurité / anti-raid (`security-bot`)
- **HIGH** — poids du score de raid `raid_weight_<similar_names|default_avatars|clustered_creation>` (40/30/30) — `security_analyzer.rs:112` · `raid_default_avatar_ratio` (0.5) · `raid_creation_spread_secs` (3600, **déjà lu côté serveur, juste absent du schéma**)
- **MED** — `raid_pattern_min_joins` (3) · `alt_ban_lookback_days` (7) / `_limit` (100) · captcha : piloter le texte du délai depuis `captcha_timeout_secs` · `lockdown_verification_level` (enum) · `raid_suggest_cooldown_secs` (300)
- **LOW** — `security_bg_poll_secs` (15)

### Économie Coude (`coude-bot`)
- **HIGH** — XP combat `combat_xp_winner_base/underdog/loser` (15/30/5) · steal `steal_pct_<afk|active>_<min|max>_pct` (10-15 / 15-25) · tout-ou-rien `win_probability`/`win_multiplier`/`loss_keep_pct` (0.5/2.0/20) · heist `base/max_success_pct` + `gain_min/max_pct` (5/55/30/75) · malédictions `curse_cost_coins`/`curse_lift_multiplier`/`leaky_wallet_fee_coins`/`fausse_assurance_fee_coins` (300/2/10/200) · `tournament_prize_pool_pct` (10)
- **MED** — durées/probas de curse (24h/0.30/1.5/10s/0.10) · cashbox `max_winners`/`active_window_days` (20/7) · `cowardice_relief_hp_pct` (20) · milestone `/repos` (niveau 15 → 8h)
- **LOW** — ~10 quotas/seuils UI (chaos, achievements, flavor, malus AFK…)

### Casino
- **wheel-bot — CRITIQUE** : **tous** les payouts/labels/poids sont en dur (`WHEEL_CASES`, 10 segments) — `wheel.rs:34-95`. → exposer un schéma `wheel_cases` (JSON) ou par-case `wheel_case_<nom>_payout`/`_weight`. + `wheel_spin_animation_ms` (4000)
- **blackjack-bot — HIGH** : `dealer_hit_threshold` (17, garde 16-20) — `blackjack_service.rs:385`. + `afk_cleanup_notification_delay_secs`
- **slot-bot — MED** : `spin_animation_total_frames` (3) / `spin_animation_frame_delay_ms` (2000)

### Voice (`voice-bot`)
- **MED** — `afk_sweep_interval_secs` (60) · `voice_ban_preset_secs` (300,3600,86400)
- **LOW** — `voice_max_user_limit` (99)

### Tickets (`ticket-bot`)
- **MED** — `ticket_subject_min/max_len` (5/100) · `ticket_desc_min/max_len` (10/2000)
- **LOW** — batchs worker close/SLA (200/100)

### Progression (`progression-bot`)
- **HIGH** — `voice_xp_tick_secs` (300, gouverne le farming XP vocal) — `mod.rs:194`
- **MED** — `streak_bonus_per_week` (0.1) / `streak_max_multiplier` (1.5) · `monthly_ranking_top_n` (10) · incohérence : commandes bot default 10 vs API 25
- **LOW** — caps XP par message/session (déjà des clamps sûrs)

### Confessions (`confessions`)
- **MED** — `thread_archive_minutes` (enum, 60) · (voir bugs de câblage §0 pour couleur/longueurs)
- **LOW** — `quota_window_hours` (24) · `report_reason_max_len` (500)

### Welcome / onboarding (`welcome-bot`)
- **HIGH** — vérification d'âge : `age_min`/`age_max` (5/120) — **aucune surface config actuellement** — `handler.rs:802`
- **MED** — `underage_ban_days` (formule ans×365) · `rejoin_title` (semble manquant) · `leave_embed_color`/`rules_embed_color`

### Worker (intervals littéraux violant le pattern config)
- **HIGH** — `automod_close_votes_secs` (60, **seul chemin qui clôt les votes à l'échéance**) · `automod_cleanup_cards_secs` (86400) · `monthly_ranking_check_secs` (3600) ~~· `tournament_check_secs`~~ (job supprimé)
- **MED** — `ai_batch_size` (5) · `announcement_publish_interval_secs` (3600) ~~· `daily_chaos_min/max_delay_secs`~~ (job supprimé)

### Autres modules
- **HIGH** — ~~Tamagotchi : effets des items de shop~~ (module supprimé) · Audit : `anomaly_detector_max_buffer_size` (500) · `watched_users_query_limit` (10000)
- **MED** — `sponsor_cooldown_secs` (30) · `role_button_cooldown_secs` (2) · seuils sprite tamagotchi
- **LOW** — poll de rotation (600s) · rate-limit purge (300ms) · poll refresh tamagotchi (60s)

---

## 2. À GARDER EN DUR — mais avec PLAFOND si un jour exposé

Ces valeurs protègent l'hôte / la correction ; si exposées, **clamper côté serveur**, jamais en entrée libre :
- Conteneurs : `pids_limit=512` (anti fork-bomb), `nofile=4096`, `nano_cpus=2 vCPU` → si `default_cpu_cores` exposé (G9), imposer un **plafond par-guild validé serveur** (comme le modèle mémoire min/max existant) · `memory_swap=RAM`, log rotation 10m/3 (clamp ≤50m/≤10).
- Invariants Docker/infra : `privileged=false`, `RCON_HOST=127.0.0.1`, labels `sentinel.managed`, drivers bridge/local/json-file — **jamais exposer**.
- Intégrité monétaire : séquencement débit→crédit, compare-and-set, invariant DB `coins>=0` — jamais exposer.
- Limites Discord : mute 28j, noms 100c, nickname 32, modal label 45, bulk-delete 14j/100 msgs.
- Structurel : courbe de niveau, `MAX_LEVEL`, RNG casino, quorum de vote.
- Garde-fous sur les nouvelles clés : `%` clampé 0..100, probas 0..1, paires min<max (steal/heist), multiplicateurs planchés (≥1.5), poids roue ≥0 avec somme>0, payouts roue clampés ±50000.

---

## 3. Ordre d'implémentation recommandé

1. **§0 — les bugs de câblage** (surtout Game Portal : tout un schéma déclaré mais jamais lu + mismatch 3/5). Valeur immédiate : ce que l'utilisateur croit déjà configurable.
2. **Worker HIGH** (W1-W4 : les 4 `spawn_periodic` en dur, dont `automod_close_votes` critique).
3. **Sensibilité "cœur"** : poids/seuils de scoring (modération+automod) et poids de raid (sécurité) — aujourd'hui on ne règle qu'un seuil sans contrôler ses entrées.
4. **Économie** : Coude HIGH + **wheel-bot** (payouts entièrement en dur) + blackjack `dealer_hit_threshold` + Tamagotchi effets de shop.
5. **Le reste par priorité** (welcome age, voice, tickets, progression, confessions, community).

Chaque clé = entrée `config_schema` (migration idempotente, calquée sur les migrations existantes) + lecture côté code (bot/core/worker), avec garde-fous du §2.
