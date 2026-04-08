import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Infraction } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { useToast } from "./useToast";

export function useInfractions() {
  const { data: infractions, loading, error, refresh: fetchInfractions } = useGuildFetch<Infraction[]>(
    "get_infractions",
    [],
  );

  const { success, error: showError } = useToast();
  const deleting = ref(false);

  async function deleteInfraction(id: string) {
    deleting.value = true;
    try {
      await invoke("delete_infraction", { id });
      await fetchInfractions();
      success("Infraction supprimee avec succes.");
    } catch (e) {
      console.error("Erreur lors de la suppression de l'infraction :", e);
      showError("Erreur lors de la suppression de l'infraction.");
    } finally {
      deleting.value = false;
    }
  }

  return { infractions, loading, error, fetchInfractions, deleting, deleteInfraction };
}
