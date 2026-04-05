import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Infraction } from "../types";
import { useGuildFetch } from "./useGuildFetch";

export function useInfractions() {
  const { data: infractions, loading, error, refresh: fetchInfractions } = useGuildFetch<Infraction[]>(
    "get_infractions",
    [],
  );

  const deleting = ref(false);

  async function deleteInfraction(id: string) {
    deleting.value = true;
    try {
      await invoke("delete_infraction", { id });
      await fetchInfractions();
    } finally {
      deleting.value = false;
    }
  }

  return { infractions, loading, error, fetchInfractions, deleting, deleteInfraction };
}
