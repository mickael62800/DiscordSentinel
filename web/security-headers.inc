# En-têtes de sécurité communs.
#
# ATTENTION (piège nginx) : `add_header` dans un bloc `location` ANNULE tous
# les add_header hérités du bloc `server`. Ce fichier doit donc être inclus
# dans le bloc `server` ET re-inclus dans chaque `location` qui déclare son
# propre add_header (ex: Cache-Control), sinon CSP/HSTS disparaissent sur
# ces réponses.
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
add_header X-Content-Type-Options "nosniff" always;
add_header X-Frame-Options "DENY" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;
# Isole la fenêtre des openers cross-origin (protège contre le tabnabbing
# inversé et certaines attaques XS-Leaks).
add_header Cross-Origin-Opener-Policy "same-origin" always;
# Désactive les APIs sensibles du navigateur (l'app n'en a pas besoin).
add_header Permissions-Policy "camera=(), microphone=(), geolocation=(), interest-cohort=()" always;
# CSP : script-src 'self' uniquement (no unsafe-inline) -> empêche
# l'exécution de tout script injecté (défense XSS forte). Vite est
# configuré avec modulePreload.polyfill=false pour ne PAS générer
# de scripts inline dans index.html.
# style-src garde 'unsafe-inline' car Vue + chart.js génèrent du style
# dynamique. connect-src 'self' wss: uniquement : en prod tous les fetch
# sont relatifs (proxy nginx) et le WS est same-host -> bloque
# l'exfiltration fetch/XHR vers un domaine attaquant en cas de XSS.
add_header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' wss:; font-src 'self' data:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; upgrade-insecure-requests" always;
