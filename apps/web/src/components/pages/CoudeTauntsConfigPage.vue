<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import {
  coudeService,
  type TauntsConfig,
} from "@/services/coudeService";
import { guildsService, type DiscordTextChannel } from "@/services/guildsService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import AppButton from "../atoms/AppButton.vue";
import AppSelect from "../atoms/AppSelect.vue";
import AppToggle from "../atoms/AppToggle.vue";
import FormField from "../atoms/FormField.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";

const { selectedGuildId } = useGuildSelector();
const { success, error: toastError } = useToast();
const { confirm } = useConfirm();

const config = ref<TauntsConfig | null>(null);
const channels = ref<DiscordTextChannel[]>([]);
const channelInput = ref("");
const enabledInput = ref(true);
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);
const removing = ref<string | null>(null);

const channelOptions = computed(() => [
  { value: "", label: "— Aucun (desactive) —" },
  ...channels.value.map((c) => ({ value: c.id, label: `# ${c.name}` })),
]);

async function fetchConfig() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  error.value = null;
  try {
    const [cfg, chans] = await Promise.all([
      coudeService.getTauntsConfig(selectedGuildId.value),
      guildsService.getTextChannels(selectedGuildId.value),
    ]);
    config.value = cfg;
    channels.value = chans;
    channelInput.value = cfg.channel_id ?? "";
    enabledInput.value = cfg.enabled;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function save() {
  if (!selectedGuildId.value) return;
  saving.value = true;
  try {
    await coudeService.updateTauntsConfig(selectedGuildId.value, {
      channel_id: channelInput.value.length > 0 ? channelInput.value : null,
      enabled: enabledInput.value,
    });
    success("Config railleries sauvegardee.");
    await fetchConfig();
  } catch (e) {
    toastError(String(e));
  } finally {
    saving.value = false;
  }
}

async function removeOptOut(userId: string) {
  if (!selectedGuildId.value) return;
  const ok = await confirm({
    title: "Retirer l'opt-out",
    message: `Forcer la reactivation des railleries pour l'utilisateur ${userId} ? Il pourra de nouveau ere raille automatiquement jusqu'a ce qu'il refasse /no-taunts on.`,
  });
  if (!ok) return;
  removing.value = userId;
  try {
    await coudeService.removeTauntOptOut(selectedGuildId.value, userId);
    success("Opt-out retire.");
    await fetchConfig();
  } catch (e) {
    toastError(String(e));
  } finally {
    removing.value = null;
  }
}

onMounted(() => {
  void fetchConfig();
});
watch(selectedGuildId, () => {
  void fetchConfig();
});
</script>

<template>
  <div class="taunts-page">
    <header class="page-header">
      <h1>🔥 Railleries automatiques</h1>
      <p class="subtitle">
        Configure le salon ou les railleries sont postees et la liste des
        joueurs qui ont opt-out via <code>/no-taunts on</code>.
      </p>
    </header>

    <LoadingState v-if="loading" message="Chargement…" />
    <ErrorState v-else-if="error" :message="error" @retry="fetchConfig" />

    <div v-else-if="config" class="card card--lg config-card">
      <h2>Configuration</h2>

      <FormField
        label="Salon des railleries"
        hint="Choisir un salon texte. Selectionner « Aucun » desactive la feature."
      >
        <AppSelect v-model="channelInput" :options="channelOptions" />
      </FormField>

      <FormField label="Active">
        <div class="toggle-row">
          <AppToggle v-model="enabledInput" />
          <span class="toggle-label">
            {{ enabledInput ? "Les railleries sont postees" : "Feature desactivee globalement" }}
          </span>
        </div>
      </FormField>

      <div class="actions">
        <AppButton :disabled="saving" @click="save">
          {{ saving ? "Sauvegarde…" : "Enregistrer" }}
        </AppButton>
      </div>
    </div>

    <div v-if="config" class="card card--lg opt-outs-card">
      <h2>Joueurs opt-out ({{ config.opt_outs.length }})</h2>
      <p class="hint">
        Ces joueurs ont tape <code>/no-taunts on</code>. Tu peux forcer le
        retrait de leur opt-out ci-dessous (ils devront re-taper la commande
        pour se re-proteger).
      </p>

      <EmptyState
        v-if="config.opt_outs.length === 0"
        message="Aucun joueur n'a opt-out."
      />

      <ul v-else class="opt-outs-list">
        <li v-for="userId in config.opt_outs" :key="userId">
          <span class="user-id">{{ userId }}</span>
          <AppButton
            variant="danger"
            :disabled="removing === userId"
            @click="removeOptOut(userId)"
          >
            {{ removing === userId ? "Retrait…" : "Retirer" }}
          </AppButton>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.taunts-page {
  max-width: 820px;
  margin: 0 auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.page-header h1 {
  margin: 0 0 8px;
  font-size: 28px;
}

.subtitle {
  margin: 0;
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 1.5;
}

/* Override : structure flex verticale pour les deux cards de la page. */
.config-card,
.opt-outs-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.config-card h2,
.opt-outs-card h2 {
  margin: 0 0 var(--space-xs);
  font-size: 18px;
}

.hint {
  margin: 0;
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
}

.hint code,
.subtitle code {
  background: var(--bg-elevated);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}

.toggle-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.toggle-label {
  font-size: 14px;
  color: var(--text-secondary);
}

.actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 8px;
}

.opt-outs-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.opt-outs-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  background: var(--bg-elevated);
  border-radius: 8px;
}

.user-id {
  font-family: var(--font-mono, monospace);
  font-size: 13px;
}
</style>
