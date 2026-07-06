/**
 * Garde-fou contre les URLs malicieuses (javascript:, data:, vbscript:, etc.)
 * dans les attributs href / src.
 *
 * Le risque : Vue n'echappe PAS les attributs `:src` / `:href`. Si une donnee
 * provenant de l'API contient `javascript:alert(1)`, elle s'execute au clic.
 *
 * Strategie : whitelist de domaines connus (Discord CDN, GitHub avatars, ...)
 * et fallback `null` -> le composant peut alors afficher un placeholder.
 */

const ALLOWED_IMAGE_HOSTS = new Set<string>([
  "cdn.discordapp.com",
  "media.discordapp.net",
  "avatars.githubusercontent.com",
]);

/**
 * Valide une URL d'image (`<img :src>`). Retourne l'URL si elle pointe vers
 * un domaine whitelist en HTTPS, sinon `null`.
 *
 * - Tout protocole non-https -> null (elimine javascript:, data:, vbscript:)
 * - Host hors whitelist -> null
 * - URL malformee -> null
 */
/**
 * Valide une URL de LIEN (`<a :href>`). Autorise uniquement http(s) — bloque
 * `javascript:`, `data:`, `vbscript:` qui s'executeraient au clic (vol de token).
 * Contrairement aux images, on n'impose pas de whitelist d'hote (un lien de
 * preuve peut pointer n'importe ou), seulement le protocole.
 */
export function safeLinkUrl(url: string | null | undefined): string | null {
  if (!url) return null;
  try {
    const u = new URL(url);
    if (u.protocol !== "https:" && u.protocol !== "http:") return null;
    return u.toString();
  } catch {
    return null;
  }
}

export function safeImageUrl(url: string | null | undefined): string | null {
  if (!url) return null;
  try {
    const u = new URL(url);
    if (u.protocol !== "https:") return null;
    if (!ALLOWED_IMAGE_HOSTS.has(u.host)) return null;
    return u.toString();
  } catch {
    return null;
  }
}
