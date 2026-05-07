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

const ALLOWED_LINK_PROTOCOLS = new Set<string>(["https:", "http:", "mailto:"]);

/**
 * Valide une URL d'image (`<img :src>`). Retourne l'URL si elle pointe vers
 * un domaine whitelist en HTTPS, sinon `null`.
 *
 * - Tout protocole non-https -> null (elimine javascript:, data:, vbscript:)
 * - Host hors whitelist -> null
 * - URL malformee -> null
 */
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

/**
 * Valide une URL pour href (`<a :href>`). Plus permissif (autorise n'importe
 * quel domaine en https/http/mailto), mais bloque toujours les protocoles
 * dangereux comme javascript:, data:, vbscript:, blob:, file:.
 */
export function safeLinkUrl(url: string | null | undefined): string | null {
  if (!url) return null;
  try {
    const u = new URL(url);
    if (!ALLOWED_LINK_PROTOCOLS.has(u.protocol)) return null;
    return u.toString();
  } catch {
    return null;
  }
}
