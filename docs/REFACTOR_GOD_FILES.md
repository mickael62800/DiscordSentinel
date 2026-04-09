# Refactoring — GOD Files

## Fichiers identifiés

### Priorité 1 — Critique

#### `services/api/src/adapters/inbound/http/handlers/coude.rs` — 2243 lignes
- **49 handlers** + **44 DTOs** dans un seul fichier
- SQL inline dans chaque handler (pas de repository)
- Mix de 6 domaines : players, combats, bets, economy, inventory, social

**Split propose :**
```
handlers/coude/
  mod.rs           # re-exports
  dto.rs           # 44 DTOs (response + request)
  players.rs       # 10 handlers — CRUD, stats, XP, class, stat points
  combats.rs       # 10 handlers — create, resolve, expire, list, pending
  bets.rs          # 5 handlers — place, resolve, refund
  economy.rs       # 10 handlers — transfer, steal, casino, coins, adjust
  inventory.rs     # 10 handlers — items, primes, insurance
  social.rs        # 9 handlers — cooldowns, leaderboard, events, utility
```

**Hexagonal complet (phase 2) :**
```
ports/outbound/coude_repository.rs    # Trait avec toutes les queries
adapters/outbound/postgres/coude_repository.rs  # Implementation PG
ports/inbound/coude/
  players.rs    # ManageCoudePlayersUseCase
  combats.rs    # ManageCoudeCombatsUseCase
  economy.rs    # ManageCoudeEconomyUseCase
application/coude/
  players_service.rs
  combats_service.rs
  economy_service.rs
```

---

### Priorité 2 — Haute

#### `bots/voice-bot/src/handlers/voice.rs` — ~750 lignes
- Channel creation + permissions + member tracking + AFK + queue + session cards + cooldowns
- 8 responsabilites dans un seul fichier

**Split propose :**
```
handlers/
  channel_lifecycle.rs   # create_channel, check_and_delete_empty, move_member
  channel_permissions.rs # set_permissions, handle_private, handle_public
  member_events.rs       # join, leave, move, deafen tracking
```

---

#### `bots/blackjack-bot/src/handler.rs` — 547 lignes
- Panel click, bet select, invite, join, close, AFK cleanup, game over check
- Mix de gestion de table et de gameplay

**Split propose :**
```
handlers/
  table_handler.rs    # panel_click, invite, join, close_table
  game_handler.rs     # bet_select, game_over_check, replay
  afk_cleanup.rs      # background task AFK
```

---

#### `bots/blackjack-bot/src/commands/blackjack.rs` — 428 lignes
- Slash command + component handler + embed builder + button builder + 20 messages fun

**Split propose :**
```
commands/
  blackjack.rs   # handle, handle_component (leger)
  embeds.rs      # build_game_embed, card_to_unicode, hand_to_string
  buttons.rs     # build_buttons
  messages.rs    # BJ_NATURAL, BJ_WIN, BJ_BUST, BJ_LOSE, pick_random
```

---

### Priorité 3 — Moyenne

#### `services/api/src/adapters/inbound/http/handlers/blackjack.rs` — 425 lignes
- Solo game handlers + multiplayer table handlers + DTOs

**Split propose :**
```
handlers/
  blackjack_game.rs    # start, hit, stand, double, get_active
  blackjack_tables.rs  # create_table, join, close, list_players, list_games
  blackjack_dto.rs     # Tous les DTOs
```

---

#### `bots/audit-bot/src/handler.rs` — ~387 lignes
- 15+ event handlers + TypeMap keys + utility functions + watched users

**Split propose :**
```
handler.rs          # EventHandler impl (dispatch seulement)
type_keys.rs        # Toutes les TypeMap keys
watched_users.rs    # is_watched, track_activity
```

---

#### `bots/coude-bot/src/commands/coude.rs` — ~350 lignes
- Command registration + combat handler + embed building

**Split propose :**
```
commands/
  coude.rs           # register + handle (leger)
  combat_embeds.rs   # build_combat_embed, build_round_embed
  combat_handler.rs  # resolve_combat_internal, handle_accept
```

---

#### `apps/desktop/src/components/pages/ComponentConfigPage.vue` — ~300 lignes
- Config display + forms + token management + toggles + bot/worker detection

**Split propose :**
```
pages/
  ComponentConfigPage.vue   # Layout + orchestration
components/
  ConfigForm.vue            # Formulaire de config (fields dynamiques)
  TokenManager.vue          # Gestion des tokens bot
  ConfigToggles.vue         # Toggles boolean groupes
```

---

#### `apps/desktop/src/components/pages/AuditPage.vue` — ~300 lignes
- Filtrage + affichage + details d'evenements varies (roles, channels, messages, voice)

**Split propose :**
```
pages/
  AuditPage.vue             # Layout + filtres
components/
  AuditEventList.vue        # Liste des events
  AuditEventDetail.vue      # Detail conditionnel par type
  AuditFilters.vue          # Filtres (date, type, actor)
```

---

## Resume

| Priorite | Fichier | Lignes | Domaines | Effort |
|----------|---------|--------|----------|--------|
| **P1** | `handlers/coude.rs` | 2243 | 6 | 2-3 jours |
| **P2** | `voice/handlers/voice.rs` | ~750 | 3 | 0.5 jour |
| **P2** | `blackjack-bot/handler.rs` | 547 | 3 | 0.5 jour |
| **P2** | `blackjack-bot/commands/blackjack.rs` | 428 | 4 | 0.5 jour |
| **P3** | `handlers/blackjack.rs` (API) | 425 | 3 | 0.5 jour |
| **P3** | `audit-bot/handler.rs` | ~387 | 3 | 0.25 jour |
| **P3** | `coude-bot/commands/coude.rs` | ~350 | 3 | 0.25 jour |
| **P3** | `ComponentConfigPage.vue` | ~300 | 3 | 0.25 jour |
| **P3** | `AuditPage.vue` | ~300 | 3 | 0.25 jour |

**Effort total estime : 5-6 jours**

## Ordre recommande

1. `coude.rs` (le plus gros impact, 2243 lignes → 7 fichiers)
2. `blackjack handler + commands` (coherent avec le refactor multijoueur en cours)
3. `voice.rs` (le plus complexe cote bot)
4. Les 4 fichiers P3 (rapides, 0.25j chacun)
