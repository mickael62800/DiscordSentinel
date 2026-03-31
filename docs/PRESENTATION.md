# DiscordSentinel — Presentation

DiscordSentinel est une plateforme complete de moderation et de gestion pour les serveurs Discord. Elle se compose de plusieurs assistants automatiques (les "bots"), d'un cerveau central qui prend les decisions, d'une application de bureau pour les administrateurs, et d'une intelligence artificielle capable de detecter les contenus problematiques.

---

## Les bots : vos assistants sur le serveur

Chaque bot a un role bien precis. Ils fonctionnent ensemble pour proteger et animer votre communaute.

---

### Automod Bot — Le gardien automatique

Ce bot surveille en permanence tous les messages envoyes sur le serveur. Il detecte automatiquement :

- **Le spam** : messages repetitifs, flood de messages, abus de majuscules
- **Les insultes** : mots offensants en francais et en anglais
- **Les liens dangereux** : tentatives de phishing, faux sites Discord ou Steam, arnaques
- **Les liens non autorises** : URLs et invitations Discord non souhaitees

Quand un probleme est detecte, le bot agit immediatement : il peut avertir l'utilisateur, supprimer le message, le rendre muet temporairement, ou proposer un bannissement selon la gravite. Si le serveur central est indisponible, il prend des decisions de securite localement.

---

### Moderation Bot — L'outil des moderateurs

Ce bot donne aux moderateurs un ensemble de commandes pour gerer les membres du serveur :

- **Avertir** (`/warn`) : envoyer un avertissement officiel a un membre avec un niveau de gravite (faible, moyen, eleve)
- **Rendre muet** (`/mute`) : empecher un membre de parler pendant une duree determinee
- **Bannir** (`/ban`) : exclure un membre du serveur, temporairement ou definitivement
- **Retirer un mute ou un ban** (`/unmute`, `/unban`) : pardonner un membre
- **Consulter l'historique** (`/history`) : voir toutes les sanctions passees d'un membre
- **Ajouter une note** (`/note`) : ecrire une observation interne sur un membre (visible uniquement par l'equipe de moderation), avec des categories : generale, avertissement, positive ou contexte

**Systeme de strikes (escalade automatique)** : Chaque sanction ajoute un "strike" au compteur du membre. Quand le nombre de strikes atteint un seuil configure (par exemple 3 strikes en 1 heure), une sanction plus severe est appliquee automatiquement (mute, puis ban). Les administrateurs configurent les seuils selon les besoins de leur serveur.

**Rappels de sanctions temporaires** : Quand un moderateur applique un mute ou un ban temporaire, le systeme programme automatiquement un rappel 1 heure avant l'expiration. Le moderateur recoit une notification pour decider s'il prolonge la sanction ou la laisse expirer.

---

### Security Bot — Le bouclier anti-raid

Ce bot protege le serveur contre les attaques coordonnees et les comptes suspects :

- **Detection de raid** : si un grand nombre de comptes rejoignent le serveur en meme temps, le bot declenche une alerte et active les protections
- **Verification des comptes** : les comptes tres recents (moins de 24 heures) sont signales comme suspects
- **Quarantaine** : les comptes suspects recoivent un role restrictif qui les empeche d'acceder au serveur normalement
- **Captcha** : le membre suspect recoit un message prive avec un bouton "Je suis humain" a cliquer pour prouver qu'il n'est pas un robot. S'il ne repond pas dans les 5 minutes, il est expulse
- **Ralentissement automatique** : pendant un raid, le bot active un delai entre les messages sur tous les salons pour limiter le spam, puis le desactive une fois le calme revenu

---

### Image Bot — L'oeil vigilant

Ce bot analyse automatiquement toutes les images partagees sur le serveur :

- Detecte les images a caractere sexuel (NSFW)
- Detecte les images de produits illicites
- Utilise l'intelligence artificielle pour classer chaque image

Si une image est jugee problematique, le bot la supprime et avertit l'utilisateur. Si le systeme d'analyse est temporairement indisponible, l'image est supprimee par precaution — c'est le bot le plus prudent de la plateforme.

---

### Progression Bot — Le suivi d'activite et de progression

Ce bot suit l'activite de chaque membre et recompense leur participation :

- **Statistiques** (`/stats`) : nombre de messages envoyes, temps passe en vocal, infractions
- **Systeme de niveaux et d'XP** : chaque message et chaque minute en vocal rapporte de l'experience. En montant de niveau, les membres peuvent debloquer des roles speciaux
- **Classements** (`/stats top`, `/level top`) : voir qui sont les membres les plus actifs du serveur
- **Annonces de niveau** : quand un membre monte de niveau, le bot le felicite publiquement

---

### Ticket Bot — Le support par tickets

Ce bot permet aux membres de demander de l'aide ou de signaler un probleme :

- **Creer un ticket** : via un panneau avec un bouton, le membre choisit le type de probleme (probleme serveur, probleme avec un membre, contestation de sanction, urgence, question, etc.)
- **Conversation privee** : un salon prive est cree entre le membre et l'equipe de moderation
- **Gestion** : les moderateurs peuvent repondre, inviter d'autres personnes, creer un appel vocal, assigner le ticket a un responsable, et le fermer quand le probleme est resolu
- **Fermeture automatique** : les tickets sans activite depuis 7 jours sont fermes automatiquement

Les moderateurs peuvent aussi repondre aux tickets depuis l'application de bureau, et les reponses apparaissent en temps reel dans Discord.

---

### Voice Bot — Le maitre des salons vocaux

Ce bot permet aux membres de creer et gerer leurs propres salons vocaux temporaires :

- **Creation automatique** : le membre rejoint un salon "createur" et un salon vocal prive est cree pour lui
- **Controle total** : le proprietaire du salon peut le renommer, le verrouiller, inviter ou expulser des membres, transferer la propriete
- **Co-administrateurs** : le proprietaire peut deleguer la gestion a d'autres membres
- **Vote kick** : les membres presents peuvent voter pour expulser quelqu'un (majorite requise)
- **Whitelist et bannissement** : autoriser ou interdire l'acces a des membres specifiques
- **File d'attente** : si le salon est plein, les membres attendent et sont deplaces automatiquement quand une place se libere
- **Detection AFK** : les membres inactifs (muet et sourd) sont automatiquement deplaces vers un salon AFK apres un delai configurable
- **Liens d'invitation** : le proprietaire peut generer un code d'invitation temporaire (valide 15 min a 24h) que n'importe qui peut utiliser pour rejoindre
- **Themes** : des modeles pre-configures (Gaming, Musique, Travail...) permettent de creer des salons avec des parametres adaptes en un clic
- **Mode presentation** : un mode ou seul le presentateur peut parler, les autres ecoutent. Ideal pour les evenements

---

### Audit Bot — Le journal de bord

Ce bot enregistre tout ce qui se passe sur le serveur pour permettre aux administrateurs de comprendre ce qui s'est passe en cas de probleme :

- Messages supprimes ou modifies
- Membres qui arrivent, partent, sont bannis
- Changements de roles, de pseudos, de permissions
- Creation et suppression de salons
- Mouvements dans les salons vocaux
- Invitations creees ou supprimees

Tout est stocke de maniere organisee et consultable depuis l'application de bureau.

---

### Community Bot — L'accueil et les roles

Ce bot gere l'accueil des nouveaux membres et la distribution des roles :

- **Auto-roles** : quand un nouveau membre rejoint le serveur, il recoit automatiquement certains roles (immediatement ou apres un delai)
- **Panneaux de roles** : des messages avec des boutons permettent aux membres de choisir eux-memes leurs roles (couleur, centres d'interet, notifications, etc.)
- **Synchronisation** : les roles Discord sont synchronises avec le systeme central pour etre visibles dans l'application de bureau

---

## Le cerveau : l'API centrale

Tous les bots envoient leurs informations a un serveur central qui :

- **Analyse les messages** avec des regles configurables et l'intelligence artificielle
- **Calcule un score de gravite** pour chaque infraction et decide de l'action appropriee
- **Stocke tout l'historique** : infractions, sanctions, tickets, statistiques, logs
- **Gere les points de conduite** : chaque membre a un capital de points qui diminue a chaque sanction et se regenere avec le temps
- **Envoie les evenements en temps reel** a l'application de bureau via WebSocket

---

## L'intelligence artificielle

Le systeme embarque deux modeles d'IA :

- **Detection d'images** : classe les images en 3 categories (normal, NSFW, illicite) avec un taux de confiance
- **Analyse de sentiments** : detecte la colere, la rage, les menaces et le harcelement dans les messages texte

Les seuils de sensibilite sont configurables par serveur : un serveur pour adultes peut tolerer plus de contenu qu'un serveur familial. Si les modeles IA ne sont pas disponibles, le systeme fonctionne quand meme en se basant uniquement sur les regles de detection classiques.

---

## L'application de bureau

Une application native (Windows, Mac, Linux) permet aux administrateurs de tout gerer depuis une interface visuelle :

- **Tableau de bord** : vue d'ensemble avec graphiques d'activite, statistiques cles
- **Infractions** : liste de toutes les infractions avec details et filtres
- **Regles** : configurer les seuils et poids de chaque type de detection
- **Bans** : liste des bannissements avec recherche
- **Moderation** : appliquer des actions et consulter l'historique
- **Securite** : suivi des evenements de securite en temps reel
- **Tickets** : gerer les demandes d'aide (repondre, assigner, fermer)
- **Salons vocaux** : voir les salons actifs et leurs parametres
- **Points de conduite** : classement et historique des points
- **Niveaux et XP** : configuration du systeme de progression et des recompenses
- **Panneaux de roles** : gerer les panneaux et les auto-roles
- **Utilisateurs surveilles** : dossiers complets avec infractions, sanctions, notes, evenements de securite
- **Audit** : consultation des logs d'activite
- **Analytics** : heatmaps d'activite, tendances de moderation, heures de pointe
- **Configuration IA** : ajuster les seuils de sensibilite de l'intelligence artificielle
- **Configuration bots** : parametrer chaque bot pour chaque serveur
- **Roles Discord** : visualiser les roles du serveur

Les notifications en temps reel sont envoyees directement sur le bureau quand un evenement important se produit (nouvelle infraction, ticket, alerte de securite).

---

## Les workers : les taches de fond

Trois services travaillent en arriere-plan pour maintenir le systeme :

- **Worker moderation** : regenere les points de conduite, nettoie les bans expires, synchronise les propositions de ban, et envoie les rappels de sanctions temporaires
- **Worker analytics** : genere des rapports d'activite quotidiens et horaires pour les graphiques et statistiques
- **Worker monitoring** : surveille la sante de tous les services

---

## En resume

| Composant | Role en une phrase |
|-----------|-------------------|
| Automod Bot | Detecte et sanctionne automatiquement les messages problematiques |
| Moderation Bot | Donne aux moderateurs les outils pour avertir, muter, bannir et documenter |
| Security Bot | Protege le serveur contre les raids et les comptes suspects |
| Image Bot | Analyse et supprime les images inappropriees grace a l'IA |
| Progression Bot | Suit l'activite des membres et recompense leur participation |
| Ticket Bot | Permet aux membres de demander de l'aide via un systeme de tickets |
| Voice Bot | Cree et gere des salons vocaux temporaires personnalisables |
| Audit Bot | Enregistre tout ce qui se passe sur le serveur |
| Community Bot | Accueille les nouveaux membres et gere les panneaux de roles |
| API centrale | Le cerveau qui analyse, decide et stocke toutes les donnees |
| Application desktop | L'interface de controle pour les administrateurs |
| Intelligence artificielle | Detecte les images et sentiments problematiques |
| Workers | Les taches automatiques en arriere-plan |
