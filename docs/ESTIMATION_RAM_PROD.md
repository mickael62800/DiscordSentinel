# Estimation RAM — Production Linux

Estimation de la consommation mémoire de DiscordSentinel en production sur un serveur Linux, binaires compilés en `--release`.

## Contexte

- **OS cible** : Linux serveur (pas de GUI, pas d'IDE)
- **Build** : Rust release (`cargo build --release`)
- **Référence** : serveur 16 GB RAM
- **Périmètre** : 16 bots + API + workers existants + workers proposés + PostgreSQL + Redis

---

## Décompte détaillé

| Composant | Quantité | RAM/unité | Total |
|---|---|---|---|
| Bots Serenity (release, Linux) | 16 | 50–90 MB | **~1.1 GB** |
| Workers existants | 6 | 15–25 MB | **~120 MB** |
| Workers proposés (cf. WORKERS_PROPOSES.md) | +10 | 15–25 MB | **~200 MB** |
| API Axum + pool SQLx | 1 | 80–150 MB | **~120 MB** |
| PostgreSQL (tuning modéré) | 1 | 200–400 MB | **~300 MB** |
| Redis | 1 | 30–80 MB | **~60 MB** |
| Linux base (systemd, sshd, kernel) | — | — | **~250 MB** |

---

## Totaux par scénario

| Scénario | RAM utilisée | % sur 16 GB |
|---|---|---|
| **Config actuelle** (sans nouveaux workers) | ~1.9 GB | **~12 %** |
| **Avec les 10 workers proposés** | ~2.1 GB | **~13 %** |
| **Pic réaliste** (gros serveurs Discord, cache chaud) | ~3 GB | **~19 %** |

**Conclusion rapide** : ~2 GB en régime normal, ~3 GB au pic. On utilise **12–19 %** d'un serveur 16 GB.

---

## Facteurs qui peuvent faire grimper la RAM

1. **Cache Serenity** — proportionnel à la taille des guilds surveillées. Un serveur Discord de 100k membres peut ajouter 100–200 MB par bot qui le cache.
2. **PostgreSQL `shared_buffers`** — si tuné à 25 % de la RAM (recommandation standard), ça peut monter à 2–4 GB dédiés à la DB.
3. **Connexions PostgreSQL** — chaque connexion ≈ 5–10 MB. Un pool généreux × N services peut représenter plusieurs centaines de MB.
4. **Logs en mémoire / tracing buffers** — si mal configurés (batching trop agressif, pas de flush).
5. **Allocateur fragmenté** — la `glibc malloc` peut fragmenter sur services long-running ; jemalloc/mimalloc résolvent ça.

---

## Marge disponible sur 16 GB

Avec ~2–3 GB utilisés par l'applicatif :

- **4 GB** libres pour tuner `shared_buffers` Postgres → perfs DB massives
- **2–4 GB** laissés au **page cache Linux** → énorme bénéfice pour la DB et les lectures disque
- **6–8 GB** totalement libres pour absorber les pics, faire des backups, tourner d'autres services

**16 GB est largement surdimensionné** pour ce workload. En pratique :

| Serveur | Viabilité |
|---|---|
| **16 GB** | ✅ Très confortable, énorme marge |
| **8 GB** | ✅ Confortable, marge suffisante |
| **4 GB** | ⚠️ Jouable mais serré aux pics, peu de marge pour tuning Postgres |
| **2 GB** | ❌ Trop juste, risque OOM |

---

## Recommandations de tuning prod

### Build Rust

```toml
# Cargo.toml workspace root
[profile.release]
lto = "fat"
codegen-units = 1
strip = true
opt-level = 3
```

Gain attendu : binaires ~30 % plus petits, **10–15 % de RAM en moins**, démarrage plus rapide.

### Allocateur mémoire

Utiliser **jemalloc** ou **mimalloc** comme allocateur global dans chaque binaire long-running :

```toml
[dependencies]
tikv-jemallocator = "0.6"
```

```rust
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

Impact : réduit drastiquement la fragmentation sur les processus qui tournent des jours/semaines (tous les bots).

### Cache Serenity

Limiter le cache à ce qui est utilisé par chaque bot via `CacheSettings` :

- `voice-bot` n'a pas besoin du cache des messages
- `moderation-bot` n'a pas besoin du cache des presences
- `blackjack-bot` n'a besoin de presque rien

Gros gain si plusieurs bots sont connectés aux mêmes guilds (très gros gain sur les guilds 50k+).

### Services partagés

- **Un seul** process PostgreSQL pour tous les services
- **Un seul** process Redis pour tous les services
- Pool de connexions SQLx raisonnable par service (5–10 max, pas 50)

### Monitoring

Exposer les métriques RAM via le `monitoring-worker` existant pour détecter tôt :
- Fuites mémoire (croissance linéaire sur plusieurs jours)
- Fragmentation (RSS >> heap alloué)
- Explosion du cache Serenity

---

## TL;DR

> DiscordSentinel complet (16 bots + API + 16 workers + DB + Redis) consomme **~2 GB en régime normal**, **~3 GB au pic** sur Linux en release.
>
> Sur un serveur 16 GB → **~13 %**, avec une marge énorme pour tuner Postgres et laisser respirer le page cache.
>
> Les 10 nouveaux workers proposés n'ajoutent que **~250 MB**, c'est négligeable.
