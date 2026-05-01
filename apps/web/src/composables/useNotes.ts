import { ref } from "vue";
import { notesService } from "@/services/notesService";
import { useGuildSelector } from "./useGuildSelector";
import { useSharedUserLookup } from "./useSharedUserLookup";
import { useToast } from "./useToast";
import type { UserNote, AddNotePayload } from "@/types/notes";

export function useNotes() {
  const { guildIdFilter } = useGuildSelector();
  const { sharedUserId } = useSharedUserLookup();
  const { success, error: showError } = useToast();

  const notes = ref<UserNote[]>([]);
  // Reuse le ref module-level partage pour rester en phase avec EvidencePage
  // et permettre au bouton du journal de pre-remplir l'ID.
  const lookupUserId = sharedUserId;
  const loading = ref(false);

  async function fetch(userId: string) {
    if (!guildIdFilter.value || !userId.trim()) {
      notes.value = [];
      return;
    }
    loading.value = true;
    try {
      notes.value = await notesService.list(guildIdFilter.value, userId.trim());
    } catch (e) {
      console.error("Erreur chargement notes :", e);
      showError("Impossible de charger les notes.");
      notes.value = [];
    } finally {
      loading.value = false;
    }
  }

  async function add(payload: AddNotePayload) {
    try {
      await notesService.add(payload);
      success("Note ajoutée.");
      await fetch(payload.user_id);
    } catch (e) {
      console.error("Erreur ajout note :", e);
      showError("Erreur lors de l'ajout.");
      throw e;
    }
  }

  async function remove(id: string) {
    try {
      await notesService.remove(id);
      notes.value = notes.value.filter((n) => n.id !== id);
      success("Note supprimée.");
    } catch (e) {
      console.error("Erreur suppression note :", e);
      showError("Erreur lors de la suppression.");
    }
  }

  return { notes, lookupUserId, loading, fetch, add, remove };
}
