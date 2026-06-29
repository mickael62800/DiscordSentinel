<script setup lang="ts">
import AppSelect from "@/components/atoms/AppSelect.vue";
import { onMounted, ref } from "vue";
import { dockerService, type DockerContainer } from "@/services/dockerService";
import { useToast } from "@/composables/useToast";

const props = defineProps<{
  container: DockerContainer;
}>();

const emit = defineEmits<{
  close: [];
}>();

const { error: showError } = useToast();

const logsContent = ref("");
const logsTail = ref(200);
const logsLoading = ref(false);

/** Extrait un message d'erreur lisible depuis une valeur `unknown` (catch). */
function errMsg(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "object" && e !== null && "message" in e) {
    return String((e as { message: unknown }).message);
  }
  return String(e);
}
function cleanName(n: string): string {
  return n.replace(/^\//, "");
}

async function fetchLogs() {
  logsLoading.value = true;
  try {
    const r = await dockerService.containerLogs(props.container.id, logsTail.value, true);
    logsContent.value = r.logs;
  } catch (e: unknown) {
    showError(`Erreur logs : ${errMsg(e)}`);
  } finally {
    logsLoading.value = false;
  }
}

onMounted(fetchLogs);
</script>

<template>
  <div class="logs-modal" @click.self="emit('close')">
    <div class="logs-window">
      <div class="logs-head">
        <strong>📋 Logs : {{ cleanName(container.names[0] ?? '') }}</strong>
        <div class="logs-controls">
          <label>Lignes :
            <AppSelect v-model.number="logsTail" @change="fetchLogs">
              <option :value="50">50</option>
              <option :value="200">200</option>
              <option :value="500">500</option>
              <option :value="2000">2000</option>
              <option :value="5000">5000</option>
            </AppSelect>
          </label>
          <button class="btn xs" :disabled="logsLoading" @click="fetchLogs">↻</button>
          <button class="btn xs" @click="emit('close')">Fermer</button>
        </div>
      </div>
      <pre v-if="!logsLoading" class="logs-body">{{ logsContent || '(vide)' }}</pre>
      <div v-else class="muted center">Chargement…</div>
    </div>
  </div>
</template>

<style scoped>
/* Buttons (réutilisés depuis la section Docker pour les contrôles du modal). */
.btn {
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}
.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.btn:disabled { opacity: 0.45; cursor: not-allowed; }
.btn.xs { padding: 3px 8px; font-size: 11px; }

/* Logs modal */
.logs-modal {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 30px;
}
.logs-window {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  width: min(1100px, 95vw);
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.logs-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
}
.logs-controls { display: flex; gap: 8px; align-items: center; font-size: 12px; }
.logs-body {
  margin: 0;
  padding: 14px 16px;
  overflow: auto;
  flex: 1;
  font-family: "JetBrains Mono", monospace;
  font-size: 11px;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-all;
  background: #0e1116;
  color: #d4d4d8;
}
.center { padding: 30px; text-align: center; }
.muted { color: var(--text-secondary); font-size: 12px; }
</style>
