# Attribution des roles par niveau XP

## Parametres du systeme
- **XP vocal** : 5 XP par minute en vocal = **300 XP/h**
- **XP texte** : 15 XP par message (cooldown 60s) = **~500-900 XP/h** selon activite
- **Formule niveau** : 5L² + 50L + 100 XP par niveau (exponentiel)

---

## Roles VOCAL (source: voice)

| Role | Heures | XP correspondant | Niveau vocal |
|------|--------|-------------------|-------------|
| 🔇 Silencieux 1h | 1h | 300 XP | **1** |
| 🔈 Murmure 5h | 5h | 1 500 XP | **4** |
| 🗣️ Bavard 10h | 10h | 3 000 XP | **7** |
| 🎙️ Causeur 25h | 25h | 7 500 XP | **11** |
| 📡 Frequence 50h | 50h | 15 000 XP | **15** |
| 🔊 Resonance 100h | 100h | 30 000 XP | **20** |
| 🎶 Onde 250h | 250h | 75 000 XP | **30** |
| 🔥 Voix d'Or 500h | 500h | 150 000 XP | **39** |
| 🌌 Eternel 5000h | 5000h | 1 500 000 XP | **91** |
| 👁️ Frequence Max 10000h | 10000h | 3 000 000 XP | **116** |

---

## Roles TEXTE (source: text)

Estimation basee sur ~500 XP/h (rythme realiste, pas 1 message/min non-stop).

| Role | Heures | XP correspondant | Niveau texte |
|------|--------|-------------------|-------------|
| ✏️ Premier Mot 1h | 1h | 500 XP | **2** |
| 📝 Curieux 5h | 5h | 2 500 XP | **6** |
| 💬 Papoteur 10h | 10h | 5 000 XP | **9** |
| 🗨️ Raconteur 25h | 25h | 12 500 XP | **14** |
| 📖 Chroniqueur 50h | 50h | 25 000 XP | **19** |
| 🖊️ Plume 100h | 100h | 50 000 XP | **25** |
| 📜 Narrateur 250h | 250h | 125 000 XP | **36** |
| 🏛️ Orateur 500h | 500h | 250 000 XP | **47** |
| 🌟 Legende Ecrite 1000h | 1000h | 500 000 XP | **61** |
| 💎 Encre Eternelle 1000h+ | 1000h+ | 500 000+ XP | **61** |

> Note : Legende Ecrite et Encre Eternelle ont le meme seuil (1000h).
> Si tu veux les differencier, mets Encre Eternelle a un niveau superieur (ex: **65** ou **70**).

---

## Roles JOURS (anciennete) - Colonne "Jours" dans l'app bureau

Gere automatiquement par le bot de progression. Le champ "Jours" = nombre de jours depuis l'arrivee sur le serveur.

| Role | Jours a saisir |
|------|---------------|
| 🕯️ Etincelle 3j | **3** |
| 🧨 Flamme 7j | **7** |
| 🌡️ Braise 15j | **15** |
| 🔥 Feu 30j | **30** |
| 💥 Brasier 90j | **90** |
| 🌋 Incendie 180j | **180** |
| ☄️ Inferno 360j | **360** |
| ⭐ Supernova 500j | **500** |
| 🌟 Etoile 1000j | **1000** |
| ☀️ Soleil 1800j | **1800** |

---

## Roles NON-XP (aucun niveau a attribuer)

Ces roles sont informatifs, attribues manuellement ou par panel de roles. Ne rien mettre.

- 👨‍🦰 homme, 👩‍🦰 femme, 🕵️‍♀️ trans mtf, 🕵️‍♂️ trans ftm
- 👶 age 15-17, 🧒 age 18-25, 🧑 age 25-29, 👨 age 30-49, 👴 age +50
- Regions (Auvergne-Rhone-Alpes, Bretagne, etc.)
- Pays (Belgique, Suisse, Quebec, etc.)
- Continents (Europe, Afrique, Asie, etc.)
- Centres d'interet (🎬 Cinephile, 🎮 Gamer, 🎵 Melomane, 📚 Lecteur, etc.)
- Personnalite (🌙 Noctambule, ☀️ Leve-tot, 😂 Humour H24, 💤 Mode Chill, 😳 Timide)
- MP (🔓 MP ouvert, 📝 MP sur demande, 🔒 MP ferme)
- 🪂 arrivage, 🌙 Membre, ⚡ Membre Actif, 🏆 Au top, 👾 PRESTIGE
- ∞ Fondateur, 👑 Admin, 🛡️ Moderateur, 🎭 Animateur, C.E.O, 🎬 Streameur, 🎤 En vocal

---

## Resume rapide a saisir dans l'app bureau

### Onglet "Roles par niveau" - Colonne VOCAL
```
Silencieux 1h      → Niveau 1
Murmure 5h         → Niveau 4
Bavard 10h         → Niveau 7
Causeur 25h        → Niveau 11
Frequence 50h      → Niveau 15
Resonance 100h     → Niveau 20
Onde 250h          → Niveau 30
Voix d'Or 500h     → Niveau 39
Eternel 5000h      → Niveau 91
Frequence Max 10kh → Niveau 116
```

### Onglet "Roles par niveau" - Colonne TEXTE
```
Premier Mot 1h     → Niveau 2
Curieux 5h         → Niveau 6
Papoteur 10h       → Niveau 9
Raconteur 25h      → Niveau 14
Chroniqueur 50h    → Niveau 19
Plume 100h         → Niveau 25
Narrateur 250h     → Niveau 36
Orateur 500h       → Niveau 47
Legende Ecrite 1kh → Niveau 61
Encre Eternelle    → Niveau 65
```

### Onglet "Roles par niveau" - Colonne JOURS
```
Etincelle 3j       → 3
Flamme 7j          → 7
Braise 15j         → 15
Feu 30j            → 30
Brasier 90j        → 90
Incendie 180j      → 180
Inferno 360j       → 360
Supernova 500j     → 500
Etoile 1000j       → 1000
Soleil 1800j       → 1800
```
