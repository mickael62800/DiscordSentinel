<script setup lang="ts">
import { ref } from "vue";
import { botTokensService } from "@/services/botTokensService";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";

const props = defineProps<{
  botName: string;
  hasToken: boolean;
}>();

const emit = defineEmits<{
  (e: "updated"): void;
}>();

const { success, error: showError } = useToast();
const { confirm } = useConfirm();

const tokenInput = ref("");
const tokenVisible = ref(false);
const saving = ref(false);
const justSaved = ref(false);

function toggleVisibility() {
  tokenVisible.value = !tokenVisible.value;
}

async function save() {
  if (!tokenInput.value) return;
  saving.value = true;
  try {
    botTokensService.save(props.botName, tokenInput.value);
    tokenInput.value = "";
    justSaved.value = true;
    setTimeout(() => (justSaved.value = false), 3000);
    emit("updated");
  } catch (e) {
    console.error("Erreur sauvegarde token:", e);
    showError("Erreur lors de la sauvegarde du token");
  } finally {
    saving.value = false;
  }
}

async function remove() {
  const ok = await confirm({
    title: "Supprimer le token",
    message: "Voulez-vous vraiment supprimer ce token de bot ?",
  });
  if (!ok) return;
  try {
    botTokensService.remove(props.botName);
    success("Token supprime avec succes");
    emit("updated");
  } catch (e) {
    console.error("Erreur suppression token:", e);
    showError("Erreur lors de la suppression du token");
  }
}
</script>

<template>
  <div class="token-section" @click.stop>
    <div v-if="hasToken" class="token-configured">
      <span class="token-status-text">Token chiffre enregistre</span>
      <button class="btn-token-delete" @click.stop="remove">Supprimer</button>
    </div>
    <div v-else class="token-input-row">
      <input
        v-model="tokenInput"
        :type="tokenVisible ? 'text' : 'password'"
        class="token-input"
        placeholder="Coller le token Discord..."
        @click.stop
      />
      <button class="btn-token-eye" @click.stop="toggleVisibility">
        {{ tokenVisible ? 'Masquer' : 'Voir' }}
      </button>
      <button
        class="btn-token-save"
        :disabled="!tokenInput || saving"
        @click.stop="save"
      >
        {{ saving ? '...' : 'Sauver' }}
      </button>
    </div>
    <span v-if="justSaved" class="token-saved-msg">Token chiffre et sauvegarde !</span>
  </div>
</template>

<style scoped>
.token-section {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--border);
}

.token-configured {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.token-status-text {
  font-size: 11px;
  color: #22c55e;
  font-weight: 500;
}

.btn-token-delete {
  font-size: 11px;
  padding: 3px 10px;
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  cursor: pointer;
  opacity: 0.7;
  transition: opacity 0.15s;
}

.btn-token-delete:hover {
  opacity: 1;
}

.token-input-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.token-input {
  flex: 1;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 12px;
  font-family: monospace;
  min-width: 0;
}

.token-input:focus {
  outline: none;
  border-color: var(--accent);
}

.token-input::placeholder {
  color: var(--text-secondary);
  opacity: 0.5;
}

.btn-token-eye {
  background: none;
  border: 1px solid var(--border);
  border-radius: 6px;
  cursor: pointer;
  font-size: 11px;
  padding: 4px 8px;
  flex-shrink: 0;
  color: var(--text-secondary);
  transition: color 0.15s;
}

.btn-token-eye:hover {
  color: var(--text-primary);
  border-color: var(--accent);
}

.btn-token-save {
  padding: 5px 12px;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
}

.btn-token-save:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.token-saved-msg {
  display: block;
  font-size: 11px;
  color: #22c55e;
  margin-top: 4px;
  font-weight: 500;
}
</style>
