/**
 * Clamp un input number (sous forme string) selon `min` / `max`.
 *
 * - Entree vide / non-numerique → retournee inchangee (le user tape, on
 *   laisse Vue valider la submission).
 * - Entree numerique hors borne → ramenee a la borne la plus proche.
 *
 * Utilise par la page de config bot pour empecher les valeurs aberrantes
 * (cf. bug `daily_snapshot_interval = 86400` au lieu de `1`).
 */
export function clampNumberValue(value: string, min?: number, max?: number): string {
  if (value === "" || value === undefined || value === null) return value;
  const n = Number(value);
  if (!Number.isFinite(n)) return value;
  let clamped = n;
  if (min !== undefined && clamped < min) clamped = min;
  if (max !== undefined && clamped > max) clamped = max;
  return String(clamped);
}
