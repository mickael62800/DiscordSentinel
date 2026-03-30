---
name: Ne jamais relancer les services sans demande explicite
description: Ne pas redémarrer/relancer les APIs, bots ou services sauf si l'utilisateur dit explicitement "relance" ou "redémarre"
type: feedback
---

Ne JAMAIS relancer, redémarrer ou toucher aux services en cours d'exécution (API, bots, ML API, desktop, workers, gateway) sauf si l'utilisateur le demande EXPLICITEMENT avec les mots "relance", "redémarre", "restart".

**Why:** L'utilisateur lance et gère ses services lui-même. Quand il dit "relance l'API ML", il veut que je relance uniquement l'API ML, pas que je lance un training ou autre action dessus.

**How to apply:** Quand l'utilisateur parle d'un service, ne faire QUE ce qu'il demande littéralement. "Relancer" = tuer + redémarrer le processus. Pas lancer un training, pas appeler des endpoints.
