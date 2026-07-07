import { httpDelete, httpGet, httpPatch, httpPost } from "@/api/http";
import type { SnapshotSummary } from "@/types";

/**
 * Gestion des snapshots guild-backup (sauvegarde des roles + salons d'un
 * serveur). La capture et la restauration sont declenchees cote bot de facon
 * asynchrone : les endpoints POST repondent 202 (accepte) sans attendre la fin.
 */
export const guildBackupService = {
  /** GET /api/guild-backup/{guild_id}/snapshots — liste des summaries. */
  listSnapshots(guildId: string): Promise<SnapshotSummary[]> {
    return httpGet(`/api/guild-backup/${guildId}/snapshots`);
  },

  /** GET /api/guild-backup/snapshots/{id} — snapshot complet (roles + salons). */
  getSnapshot(id: string): Promise<unknown> {
    return httpGet(`/api/guild-backup/snapshots/${id}`);
  },

  /** PATCH /api/guild-backup/snapshots/{id} — renomme le snapshot. */
  rename(id: string, label: string): Promise<void> {
    return httpPatch(`/api/guild-backup/snapshots/${id}`, { label });
  },

  /** DELETE /api/guild-backup/snapshots/{id} — supprime le snapshot (204). */
  remove(id: string): Promise<void> {
    return httpDelete(`/api/guild-backup/snapshots/${id}`);
  },

  /**
   * POST /api/guild-backup/{guild_id}/capture — declenche une capture async
   * cote bot (202). Le snapshot apparaitra dans la liste apres traitement.
   */
  capture(guildId: string, label?: string): Promise<void> {
    return httpPost(`/api/guild-backup/${guildId}/capture`, label ? { label } : {});
  },

  /**
   * POST /api/guild-backup/snapshots/{id}/restore — declenche une restauration
   * async cote bot (202). `wipe=true` vide d'abord le serveur (destructif).
   */
  restore(id: string, wipe: boolean): Promise<void> {
    return httpPost(`/api/guild-backup/snapshots/${id}/restore`, { wipe });
  },
};
