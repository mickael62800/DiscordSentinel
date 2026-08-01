<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import { computed, ref } from "vue";
import AppModal from "@/components/atoms/AppModal.vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { systemService } from "@/services/systemService";

const { selectedGuild, selectedGuildId } = useGuildSelector();
const { success, error: showError } = useToast();

// Double confirmation : etape 1 = avertissement + options, etape 2 = saisie du
// nom exact du serveur. Le backend re-verifie le nom (garde-fou cote use case).
const modalOpen = ref(false);
const step = ref<1 | 2>(1);
const confirmText = ref("");
const submitting = ref(false);
const opts = ref({ unban: true, unmute: true, remove_roles: true });

const guildName = computed(() => (selectedGuild.value as { name?: string } | null)?.name ?? "");
const canConfirm = computed(
  () => !!guildName.value && confirmText.value.trim() === guildName.value && !submitting.value,
);

function openDialog() {
  confirmText.value = "";
  step.value = 1;
  opts.value = { unban: true, unmute: true, remove_roles: true };
  modalOpen.value = true;
}

function closeDialog() {
  if (submitting.value) return;
  modalOpen.value = false;
}

async function doReset() {
  if (!selectedGuildId.value || !canConfirm.value) return;
  submitting.value = true;
  try {
    const res = await systemService.resetGuild(selectedGuildId.value, confirmText.value.trim(), opts.value);
    success(`Serveur réinitialisé : ${res.total_rows} lignes supprimées (${res.tables_wiped} tables).`);
    modalOpen.value = false;
  } catch (e) {
    showError(`Échec de la réinitialisation : ${e}`);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <section class="danger-zone">
    <h2>☢️ Zone de danger</h2>

    <div class="danger-card">
      <div class="warn-banner">
        <strong>⚠️ Réinitialisation totale du serveur — IRRÉVERSIBLE</strong>
      </div>
      <p class="warn-text">
        Cette action <strong>supprime définitivement TOUTES les données</strong> de
        <strong>{{ guildName || "ce serveur" }}</strong> : infractions, historiques de
        modération, reviews/votes automod, tickets, niveaux/XP, stats,
        salons vocaux, <strong>et la configuration</strong> (salons, règles, etc.).
        En parallèle, le bot <strong>débannit tout le monde</strong>, lève les mutes et
        retire les rôles temporaires/quarantaine.
      </p>
      <p class="warn-text">
        Seuls survivent l'enregistrement du serveur dans le dashboard, les accès
        web (RBAC) et les sauvegardes (snapshots).
      </p>
      <p class="warn-text danger">
        Il n'y a <strong>aucune corbeille</strong> et <strong>aucun retour arrière</strong>.
        Réservé au <strong>propriétaire</strong> du serveur.
      </p>

      <AppButton variant="danger" @click="openDialog">
        Réinitialiser complètement ce serveur…
      </AppButton>
    </div>

    <AppModal
      :visible="modalOpen"
      :title="step === 1 ? 'Réinitialiser complètement ce serveur ?' : 'Confirmation finale'"
      size="md"
      :close-on-overlay="!submitting"
      :close-on-esc="!submitting"
      @close="closeDialog"
    >
      <template v-if="step === 1">
        <p class="modal-warn">
          Toutes les données de <strong>{{ guildName }}</strong> vont être
          <strong>définitivement effacées</strong> (modération, tickets, niveaux,
          stats, configuration…). Cette action est <strong>irréversible</strong>.
        </p>
        <label class="opt"><input type="checkbox" v-model="opts.unban" /> Débannir tous les membres bannis</label>
        <label class="opt"><input type="checkbox" v-model="opts.unmute" /> Lever tous les mutes (timeouts)</label>
        <label class="opt"><input type="checkbox" v-model="opts.remove_roles" /> Retirer les rôles temporaires / quarantaine</label>
      </template>

      <template v-else>
        <p class="modal-warn">
          Pour confirmer, tape exactement le nom du serveur :
          <code>{{ guildName }}</code>
        </p>
        <input
          v-model="confirmText"
          class="confirm-input"
          :placeholder="guildName"
          autocomplete="off"
          spellcheck="false"
          @keyup.enter="canConfirm && doReset()"
        />
      </template>

      <template #footer>
        <button class="btn-ghost" :disabled="submitting" @click="closeDialog">Annuler</button>
        <AppButton variant="danger" v-if="step === 1"  @click="step = 2">
          Je comprends, continuer
        </AppButton>
        <AppButton variant="danger" v-else  :disabled="!canConfirm" @click="doReset">
          {{ submitting ? "Suppression…" : "Tout supprimer définitivement" }}
        </AppButton>
      </template>
    </AppModal>
  </section>
</template>

<style scoped>
.danger-zone { margin-top: 28px; }
.danger-zone h2 { font-size: 1.1rem; margin-bottom: 10px; color: var(--danger); }
.danger-card {
  border: 1px solid var(--danger);
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--danger) 8%, var(--bg-secondary));
  padding: 18px;
}
.warn-banner {
  background: var(--danger);
  color: #fff;
  padding: 8px 12px;
  border-radius: var(--radius-sm);
  margin-bottom: 12px;
  font-size: 0.95rem;
}
.warn-text { font-size: 0.9rem; line-height: 1.55; margin: 0 0 10px; color: var(--text-primary); }
.warn-text.danger { color: var(--danger); }
.btn-danger {
  background: var(--danger); color: #fff; border: none; border-radius: var(--radius-sm);
  padding: 10px 18px; font-weight: 700; cursor: pointer;
}
.btn-danger:disabled { opacity: 0.45; cursor: not-allowed; }
.btn-ghost {
  background: transparent; color: var(--text-secondary);
  border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 10px 18px; cursor: pointer;
}
.modal-warn { font-size: 0.92rem; line-height: 1.55; margin: 0 0 12px; }
.modal-warn code { background: var(--bg-card); padding: 2px 8px; border-radius: var(--radius-sm); color: var(--danger); font-weight: 700; }
.opt { font-size: 0.88rem; display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.confirm-input {
  width: 100%;
  background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius-sm);
  padding: 9px 12px; color: var(--text-primary); font-size: 0.95rem;
}
</style>
