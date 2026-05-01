# TODO — Surveillance & sécurité serveur (page Sécurité serveur)

Liste exhaustive des features qu'on avait identifiées pour la page de
surveillance serveur, avec leur état d'implémentation.

Audit initial : 2026-05-01.

---

## ✅ Implémenté

### Détection d'attaques actives
- **Top IPs par requêtes** (1h / 24h / 7j) — Top des IPs avec compteurs total + échecs
- **Échecs d'authentification** (401/403) — Liste détaillée avec IP, route, user-agent
- **IPs bannies fail2ban** — Lecture du fichier export host (`/var/lib/sentinel/fail2ban-status.json`)

### Surveillance proactive
- **Audit log unifié** — Lecture de la table `audit_logs` avec filtres prefix
- **Bouton "Tout nettoyer"** — Purge logs API + audit logs avec confirmation

### Hygiène
- **Expiration TLS cert Let's Encrypt** — Affichage notBefore/notAfter + badge si <14j

---

## 🔴 Priorité haute (à implémenter)

### 1. Tentatives SSH échouées
**Description** : Parse `journalctl _SYSTEMD_UNIT=ssh.service` ou `/var/log/auth.log` pour
remonter les tentatives de login SSH ratées (user, IP, count).

**Approche** : pattern fichier-shim comme fail2ban — un script host génère
`/var/lib/sentinel/ssh-failures.json` toutes les 5 min, l'API le lit.

**Effort** : ~1h30

---

### 2. Patterns suspects nginx (scanners, SQLi tentés)
**Description** : Détecter dans les logs nginx les requêtes louches : `?id=' OR 1=1`,
`/wp-admin`, `/.env`, `/admin.php`, scanners (UA `nuclei`, `nikto`, etc.).

**Approche** : modifier `nginx.conf` pour activer access_log JSON, puis worker
qui lit + grep les patterns + insère dans une nouvelle table `suspicious_requests`.

**Effort** : ~2h

---

### 3. Géolocalisation IPs suspectes
**Description** : Quand une IP apparaît dans Top IPs ou Échecs auth, afficher
le pays + l'ASN (organisation hébergeur).

**Approche** : utiliser `MaxMind GeoLite2` (DB gratuite) téléchargée au démarrage.
Crate Rust `maxminddb` pour lookup. Cache en mémoire (la DB fait ~70MB).

**Effort** : ~1h30

---

## 🟡 Priorité moyenne

### 4. Trafic anormal (graphe + alerte si pic)
**Description** : Graphe req/s sur 24h, alerte rouge si pic > 3× moyenne mobile.

**Approche** : agréger depuis `logs` table par tranche de 5 min, retourner JSON
au frontend, render avec Chart.js (déjà dispo). Calculer la moyenne mobile sur 1h.

**Effort** : ~1h

---

### 5. Connexions actives temps réel
**Description** : `ss -tn state established` → liste des connexions TCP en cours
(qui parle au serveur en ce moment).

**Approche** : pattern fichier-shim — script host écrit `/var/lib/sentinel/connections.json`.
L'API lit + filtre les internals.

**Effort** : ~1h

---

### 6. Connexions outbound (qui contact qui)
**Description** : Voir quels IPs externes l'API/bot contactent. Utile pour
détecter une exfiltration.

**Approche** : `ss -tn` filtré sur les conteneurs Docker (PIDs). Pattern fichier-shim.

**Effort** : ~2h (plus complexe car PIDs changeants)

---

### 7. Vulns Docker images (Trivy scan)
**Description** : Scanner régulier des images Docker pour CVEs connues.

**Approche** : cron host qui exécute `trivy image discordsentinel-api:latest --format json`
et écrit dans `/var/lib/sentinel/trivy.json`. L'API affiche la liste.

**Effort** : ~1h30 (+ Trivy à installer côté host)

---

### 8. Open ports check (nmap externe)
**Description** : Vérification périodique depuis l'extérieur que seuls 80/443
sont ouverts (détecte les drifts de config firewall).

**Approche** : un service externe (ex: shodan API, ou propre VPS scanner).
Plus simple : cron host qui fait `nmap -p- localhost` et compare avec une liste blanche.

**Effort** : ~2h

---

## 🟢 Priorité basse

### 9. Last successful logins
**Description** : 10 derniers logins Discord OAuth réussis (qui / quand / IP).

**Approche** : table `successful_logins` populée au callback OAuth. Frontend list.

**Effort** : ~1h

---

### 10. TLS handshake errors
**Description** : Compteur de SSL handshake échoués (signe de scan TLS / botnet).

**Approche** : enable nginx error_log debug, parser les erreurs SSL.

**Effort** : ~1h

---

### 11. Espace disque tendance
**Description** : Graphe utilisation disque sur 7 jours, alerte si croissance
anormale (peut signaler log flood ou data exfiltration).

**Approche** : cron `df -h` toutes les heures vers `/var/lib/sentinel/disk-history.json`.

**Effort** : ~1h

---

### 12. Intégrité fichiers critiques
**Description** : SHA256 des fichiers config critiques (nginx.conf, docker-compose,
.env), alerte si modifié.

**Approche** : cron `sha256sum /etc/nginx/conf.d/*.conf > /var/lib/sentinel/integrity.json`.
L'API compare au baseline.

**Effort** : ~1h30

---

### 13. Changements conteneurs inattendus
**Description** : Alerte si un conteneur a redémarré X fois ou changé d'image
sans intervention manuelle.

**Approche** : worker qui poll `docker ps` toutes les 5 min, détecte les diffs,
log en audit.

**Effort** : ~2h

---

## 🛡️ Protections actives

### 14. Auto-ban IP via API
**Description** : Endpoint `POST /api/security/ban-ip` qui ajoute une IP à la
blocklist iptables/ufw. Bouton "Bannir cette IP" sur Top IPs.

**Approche** : pattern fichier-shim — l'API écrit dans `/var/lib/sentinel/bans-pending.txt`,
un cron host applique avec `ufw deny from <IP>`.

**Effort** : ~2h

---

### 15. Rate limit dynamique avec ban automatique
**Description** : Si une IP fait > 100 req/min, ban auto pendant 15 min.

**Approche** : étendre `middleware/rate_limit.rs` pour persister les violations
dans Redis. Au-delà du seuil, append à `/var/lib/sentinel/bans-pending.txt`.

**Effort** : ~2h30

---

### 16. Alerte Discord/email
**Description** : Quand un seuil critique est dépassé (10+ échecs auth d'une
même IP en 5min, conteneur down, etc.), envoyer une notif sur un channel
Discord ou par email.

**Approche** : worker `alert-worker` qui poll les conditions critiques toutes
les minutes et publie sur un channel Discord configuré.

**Effort** : ~2-3h

---

## 📋 Plan d'action recommandé

**Quick wins (~3h total)** :
- #1 SSH failures (pattern shim, simple)
- #4 Trafic anormal graphe (data déjà en BDD)
- #11 Espace disque tendance (cron simple)

**Vraies protections (~5-6h)** :
- #14 Auto-ban IP (vraie défense active)
- #16 Alerte Discord (visibilité quand qqch arrive)
- #15 Rate limit dynamique (renforce auto-ban)

**Hardening avancé (~6h)** :
- #2 Patterns suspects nginx (détection scanner)
- #3 Géolocalisation IPs (contexte)
- #7 Trivy scan vulns (CVE images)

---

## Notes techniques

### Pattern fichier-shim
La majorité des features nécessitent de l'info HOST (fail2ban, ss, journalctl,
nmap, df). L'API étant en conteneur, on utilise le pattern :

1. **Cron host** écrit l'info dans `/var/lib/sentinel/<feature>.json`
2. **Volume** `/var/lib/sentinel:/var/lib/sentinel:ro` monté sur l'API
3. **API** lit le fichier, parse, expose via endpoint

Avantages :
- Pas besoin d'élever les caps Docker (NET_ADMIN, etc.)
- Pas d'exec dans le host depuis le conteneur (sécurité)
- Données fraîches (cron toutes les 1-5 min selon la feature)
- Si fichier absent → endpoint affiche les instructions de setup

Le script `infra/scripts/setup-host-security.sh` automatise les setup host.
