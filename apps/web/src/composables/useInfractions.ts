import { ref } from "vue";
import type { Infraction } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { useToast } from "./useToast";
import { infractionsService } from "@/services/infractionsService";

export function useInfractions() {
  const { data: infractions, loading, error, refresh: fetchInfractions } = useGuildFetch<Infraction[]>(
    (guildId) => infractionsService.getAll(guildId),
    [],
    { label: "infractions" },
  );

  const { success, error: showError } = useToast();
  const deleting = ref(false);
  const purging = ref(false);

  async function deleteInfraction(id: string) {
    deleting.value = true;
    try {
      await infractionsService.remove(id);
      await fetchInfractions();
      success("Infraction supprimee avec succes.");
    } catch (e) {
      console.error("Erreur lors de la suppression de l'infraction :", e);
      showError("Erreur lors de la suppression de l'infraction.");
    } finally {
      deleting.value = false;
    }
  }

  async function purgeAll(guildId: string): Promise<number> {
    purging.value = true;
    try {
      const result = await infractionsService.purgeAll(guildId);
      await fetchInfractions();
      success(`${result.deleted} infraction(s) supprimee(s) de la BDD.`);
      return result.deleted;
    } catch (e) {
      console.error("Erreur lors de la purge des infractions :", e);
      showError("Erreur lors de la purge des infractions.");
      throw e;
    } finally {
      purging.value = false;
    }
  }

  return { infractions, loading, error, fetchInfractions, deleting, deleteInfraction, purging, purgeAll };
}
