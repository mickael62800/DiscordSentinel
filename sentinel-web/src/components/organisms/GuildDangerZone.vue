<script setup lang="ts">
import { computed, ref } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { systemService } from "@/services/systemService";

const { selectedGuild, selectedGuildId } = useGuildSelector();
const { success, error: showError } = useToast();

const open = ref(false);
const confirmText = ref("");
const submitting = ref(false);
const opts = ref({ unban: true, unmute: true, remove_roles: true });

const guildName = computed(() => (selectedGuild.value as { name?: string } | null)?.name ?? "");
const canConfirm = computed(
  () => !!guildName.value && confirmText.value.trim() === guildName.value && !submitting.value,
);

function openDialog() {
  confirmText.value = "";
  opts.value = { unban: true, unmute: true, remove_roles: true };
  open.value = true;
}

async function doReset() {
  if (!selectedGuildId.value || !canConfirm.value) return;
  submitting.value = true;
  try {
    const res = await systemService.resetGuild(selectedGuildId.value, confirmText.value.trim(), opts.value);
    success(`Serveur réinitialisé : ${res.total_rows} lignes supprimées (${res.tables_wiped} tables).`);
    open.value = false;
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
        modération, reviews/votes automod, tickets, niveaux/XP, stats, données de jeu,
        salons vocaux, <strong>et la configuration</strong> (salons, règles, etc.).
        En parallèle, le bot <strong>débannit tout le monde</strong>, lève les mutes et
        retire les rôles temporaires/quarantaine.
      </p>
      <p class="warn-text danger">
        Il n'y a <strong>aucune corbeille</strong> et <strong>aucun retour arrière</strong>.
        Réservé au <strong>propriétaire</strong> du serveur.
      </p>

      <button v-if="!open" class="btn-danger" @click="openDialog">
        Réinitialiser ce serveur…
      </button>

      <div v-else class="confirm-box">
        <label class="opt"><input type="checkbox" v-model="opts.unban" /> Débannir tous les membres bannis</label>
        <label class="opt"><input type="checkbox" v-model="opts.unmute" /> Lever tous les mutes (timeouts)</label>
        <label class="opt"><input type="checkbox" v-model="opts.remove_roles" /> Retirer les rôles temporaires / quarantaine</label>

        <p class="confirm-instr">
          Pour confirmer, tape exactement le nom du serveur :
          <code>{{ guildName }}</code>
        </p>
        <input
          v-model="confirmText"
          class="confirm-input"
          :placeholder="guildName"
          autocomplete="off"
        />
        <div class="actions">
          <button class="btn-ghost" @click="open = false" :disabled="submitting">Annuler</button>
          <button class="btn-danger" :disabled="!canConfirm" @click="doReset">
            {{ submitting ? "Suppression…" : "Tout supprimer définitivement" }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.danger-zone { margin-top: 28px; }
.danger-zone h2 { font-size: 1.1rem; margin-bottom: 10px; color: #E74C3C; }
.danger-card {
  border: 1px solid #E74C3C;
  border-radius: 10px;
  background: color-mix(in srgb, #E74C3C 8%, var(--bg-secondary));
  padding: 18px;
}
.warn-banner {
  background: #E74C3C;
  color: #fff;
  padding: 8px 12px;
  border-radius: 6px;
  margin-bottom: 12px;
  font-size: 0.95rem;
}
.warn-text { font-size: 0.9rem; line-height: 1.55; margin: 0 0 10px; color: var(--text-primary); }
.warn-text.danger { color: #E74C3C; }
.btn-danger {
  background: #E74C3C; color: #fff; border: none; border-radius: 6px;
  padding: 10px 18px; font-weight: 700; cursor: pointer;
}
.btn-danger:disabled { opacity: 0.45; cursor: not-allowed; }
.btn-ghost {
  background: transparent; color: var(--text-secondary);
  border: 1px solid var(--border); border-radius: 6px; padding: 10px 18px; cursor: pointer;
}
.confirm-box { margin-top: 8px; display: flex; flex-direction: column; gap: 8px; }
.opt { font-size: 0.88rem; display: flex; align-items: center; gap: 8px; }
.confirm-instr { font-size: 0.88rem; margin: 8px 0 4px; }
.confirm-instr code { background: var(--bg-card); padding: 2px 8px; border-radius: 4px; color: #E74C3C; font-weight: 700; }
.confirm-input {
  background: var(--bg-card); border: 1px solid var(--border); border-radius: 6px;
  padding: 9px 12px; color: var(--text-primary); font-size: 0.95rem;
}
.actions { display: flex; gap: 10px; margin-top: 6px; }
</style>
