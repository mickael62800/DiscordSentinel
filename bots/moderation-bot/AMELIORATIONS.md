# Améliorations proposées — moderation-bot

## Analyse de l'existant

**Architecture** : Bot Serenity 0.12 centré sur la modération **manuelle** (13 slash commands), avec auto-escalation warn→mute→ban, mode apprenti (approbations), notes internes, salons de convocation, appels et export. Logging via API backend + listener Redis.

**Couverture déjà assurée par d'autres bots du monorepo** : la détection raid/spam/toxicité est gérée par `automod-bot`, `security-bot` et `cleanup-bot` — inutile de dupliquer ces fonctionnalités ici.

### Commandes existantes

| Commande | Rôle |
|---|---|
| `/warn` | Avertissement avec gravité (low/medium/high) + auto-escalation |
| `/mute` `/unmute` | Timeouts permanents ou temporaires (max 28j Discord) |
| `/ban` `/unban` | Bans permanents/temporaires avec suppression de messages |
| `/massmute` `/massban` | Actions en masse (jusqu'à 200 utilisateurs) |
| `/history` | Historique des sanctions d'un utilisateur |
| `/note` | Notes internes catégorisées |
| `/call` | Salon privé de convocation |
| `/appeal` | Ticket d'appel automatique via l'API |
| `/export` | Export historique en JSON/CSV |
| `/context` | Messages avant/après un message cible |

---

## Ajouts à forte valeur

### 1. Rappels & expirations actives
Scheduler interne qui DM le modérateur 24h avant expiration d'un mute/ban temporaire et poste automatiquement un message "sanction expirée" dans le salon de logs. Aujourd'hui les timeouts expirent silencieusement.

### 2. `/evidence` — preuves attachées
Attacher screenshots/liens de messages à une action existante (`action_id`). Crucial quand un appel arrive plusieurs semaines après la sanction et que les messages ont disparu.

### 3. `/review` — file de relecture
File des actions à relire par un senior (faux positifs, plaintes). Remplace les discussions ad-hoc et alimente des stats de qualité par modérateur.

### 4. Confirmation interactive sur cibles "à risque"
Bouton de confirmation quand la cible est :
- un autre modérateur,
- un compte > 1 an d'ancienneté sur le serveur,
- un user déjà sous appel ouvert.

Garde-fou anti-erreur.

### 5. `/compare` — historique croisé
Comparer l'historique de 2-3 utilisateurs côte-à-côte pour détecter alts/coordination (complément du security-bot qui détecte mais ne présente pas).

### 6. Templates de sanction composables
Aujourd'hui `reason_templates` = texte simple. Étendre à des **presets complets** (raison + durée + gravité + DM custom) sélectionnables en 1 clic — gros gain de temps sur les cas répétitifs.

### 7. `/modstats` — métriques par modérateur
Nombre d'actions, taux d'appel, taux d'overturn, temps de réponse moyen. Permet le coaching des apprentis.

### 8. Transcript des call rooms
Quand `/call` ouvre un salon, transcript automatique posté dans l'API à la fermeture (actuellement les conversations sont perdues).

---

## Priorisation suggérée

| Priorité | Fonctionnalité | Effort | Impact |
|---|---|---|---|
| 🔴 Haute | Rappels & expirations actives | Moyen | Élevé |
| 🔴 Haute | Templates composables | Faible | Élevé |
| 🟡 Moyenne | `/evidence` | Moyen | Élevé |
| 🟡 Moyenne | Confirmation sur cibles à risque | Faible | Moyen |
| 🟡 Moyenne | Transcript call rooms | Moyen | Moyen |
| 🟢 Basse | `/review` | Élevé | Moyen |
| 🟢 Basse | `/modstats` | Moyen | Moyen |
| 🟢 Basse | `/compare` | Faible | Faible |
