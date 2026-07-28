<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { guildBackupService } from "@/services/guildBackupService";
import type { SnapshotSummary, TableColumn } from "@/types";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useMyRole } from "@/composables/useMyRole";
import { useConfirm } from "@/composables/useConfirm";
import { useToast } from "@/composables/useToast";
import { useFormatDate } from "@/composables/useFormatDate";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import DataTable from "../organisms/DataTable.vue";
import AppButton from "../atoms/AppButton.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";

const router = useRouter();
const { selectedGuildId, selectedGuild } = useGuildSelector();
const { role, isSuper } = useMyRole();
const { confirm } = useConfirm();
const { success, error: toastError, info } = useToast();
const { formatDateTime } = useFormatDate();

// RBAC : seuls le superadmin et l'owner de la guild peuvent declencher des
// actions (capture / restore / rename / delete). Les autres roles ont une
// vue en lecture seule.
const canManage = computed(() => isSuper.value || role.value === "owner");

const snapshots = ref<SnapshotSummary[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const busy = ref(false);

const columns: TableColumn[] = [
  { key: "label", label: "Nom" },
  { key: "created_at", label: "Date" },
  { key: "role_count", label: "Rôles" },
  { key: "channel_count", label: "Salons" },
  { key: "actions", label: "" },
];

// Edition inline du label
const editingId = ref<string | null>(null);
const editLabel = ref("");

// Capture
const showCaptureModal = ref(false);
const captureLabel = ref("");

// Restore (avec double confirmation si wipe)
const showRestoreModal = ref(false);
const restoreTarget = ref<SnapshotSummary | null>(null);
const restoreWipe = ref(false);
const wipeConfirmText = ref("");
const restoreConfirmed = ref(false);

const guildName = computed(() => selectedGuild.value?.name ?? "");
// Pour valider la 2e confirmation destructive : l'utilisateur doit retaper
// le nom exact du serveur.
const wipeTextMatches = computed(
  () => wipeConfirmText.value.trim() === guildName.value.trim() && guildName.value.length > 0,
);

async function fetchSnapshots() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  error.value = null;
  try {
    snapshots.value = await guildBackupService.listSnapshots(selectedGuildId.value);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

// ── Renommer (inline) ──
function startEdit(snap: SnapshotSummary) {
  editingId.value = snap.id;
  editLabel.value = snap.label;
}
function cancelEdit() {
  editingId.value = null;
  editLabel.value = "";
}
async function saveEdit(snap: SnapshotSummary) {
  const label = editLabel.value.trim();
  if (!label || label === snap.label) {
    cancelEdit();
    return;
  }
  busy.value = true;
  try {
    await guildBackupService.rename(snap.id, label);
    success("Snapshot renommé");
    cancelEdit();
    await fetchSnapshots();
  } catch (e) {
    toastError(`Renommage : ${String(e)}`);
  } finally {
    busy.value = false;
  }
}

// ── Supprimer ──
async function removeSnapshot(snap: SnapshotSummary) {
  const ok = await confirm({
    title: "Supprimer le snapshot",
    message: `Supprimer définitivement le snapshot « ${snap.label} » (${snap.role_count} rôles, ${snap.channel_count} salons) ? Cette sauvegarde sera perdue.`,
  });
  if (!ok) return;
  busy.value = true;
  try {
    await guildBackupService.remove(snap.id);
    success("Snapshot supprimé");
    await fetchSnapshots();
  } catch (e) {
    toastError(`Suppression : ${String(e)}`);
  } finally {
    busy.value = false;
  }
}

// ── Capturer maintenant ──
function openCaptureModal() {
  captureLabel.value = "";
  showCaptureModal.value = true;
}
async function runCapture() {
  if (!selectedGuildId.value) return;
  busy.value = true;
  try {
    await guildBackupService.capture(selectedGuildId.value, captureLabel.value.trim() || undefined);
    showCaptureModal.value = false;
    info("Capture lancée — cela peut prendre un moment. Rafraîchissez la liste.");
    // Refresh optimiste apres un court delai (la capture est async cote bot).
    setTimeout(fetchSnapshots, 2500);
  } catch (e) {
    toastError(`Capture : ${String(e)}`);
  } finally {
    busy.value = false;
  }
}

// ── Aperçu du contenu d'un snapshot (avant restauration) ──
interface SnapshotPreview {
  roles?: { name: string }[];
  categories?: { name: string }[];
  channels?: { name: string; kind?: string }[];
}
const preview = ref<SnapshotPreview | null>(null);
const previewLoading = ref(false);

async function loadPreview(id: string) {
  preview.value = null;
  previewLoading.value = true;
  try {
    preview.value = (await guildBackupService.getSnapshot(id)) as SnapshotPreview;
  } catch {
    preview.value = null;
  } finally {
    previewLoading.value = false;
  }
}

// ── Restaurer ──
function openRestoreModal(snap: SnapshotSummary) {
  restoreTarget.value = snap;
  restoreWipe.value = false;
  wipeConfirmText.value = "";
  restoreConfirmed.value = false;
  showRestoreModal.value = true;
  void loadPreview(snap.id);
}
function closeRestoreModal() {
  showRestoreModal.value = false;
  restoreTarget.value = null;
}
async function runRestore() {
  const snap = restoreTarget.value;
  if (!snap) return;
  // Garde-fou : si wipe est coche, on exige la double confirmation explicite
  // (case restoreConfirmed + saisie du nom exact du serveur).
  if (restoreWipe.value && (!restoreConfirmed.value || !wipeTextMatches.value)) return;

  busy.value = true;
  try {
    await guildBackupService.restore(snap.id, restoreWipe.value);
    closeRestoreModal();
    info(
      restoreWipe.value
        ? "Restauration avec wipe lancée — action destructive en cours, cela peut prendre plusieurs minutes."
        : "Restauration lancée — cela peut prendre un moment.",
    );
  } catch (e) {
    toastError(`Restauration : ${String(e)}`);
  } finally {
    busy.value = false;
  }
}

// ── Export / Import (clonage cross-serveur) ──
async function exportSnapshot(snap: SnapshotSummary) {
  busy.value = true;
  try {
    const full = await guildBackupService.getSnapshot(snap.id);
    const blob = new Blob([JSON.stringify(full, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    const safe = snap.label.replace(/[^\w.-]+/g, "_").slice(0, 40) || "snapshot";
    a.download = `sentinel-backup-${safe}-${snap.id.slice(0, 8)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  } catch (e) {
    toastError(`Export : ${String(e)}`);
  } finally {
    busy.value = false;
  }
}

const importInput = ref<HTMLInputElement | null>(null);
function triggerImport() {
  importInput.value?.click();
}
async function onImportFile(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = ""; // reset pour permettre de ré-importer le même fichier
  if (!file || !selectedGuildId.value) return;
  busy.value = true;
  try {
    const text = await file.text();
    const snapshot = JSON.parse(text);
    await guildBackupService.importSnapshot(selectedGuildId.value, snapshot);
    success("Sauvegarde importée dans ce serveur. Restaurez-la pour l'appliquer.");
    await fetchSnapshots();
  } catch (e) {
    toastError(
      `Import : ${e instanceof SyntaxError ? "fichier JSON invalide" : String(e)}`,
    );
  } finally {
    busy.value = false;
  }
}

function goToConfig() {
  // Le formulaire de config est schema-driven (page Composants). On y renvoie ;
  // le composant guild-backup-bot y est selectionnable.
  router.push({ path: "/component-config" });
}

watch(selectedGuildId, fetchSnapshots);
onMounted(fetchSnapshots);
</script>

<template>
  <AdminPageShell title="Sauvegardes serveur" icon="💾" width="wide">
    <template #lede>
      Snapshots des rôles et salons du serveur — capture, restauration et gestion.
    </template>
    <template #actions>
      <AppButton variant="secondary" @click="goToConfig">⚙ Configurer</AppButton>
      <AppButton variant="secondary" :disabled="loading" @click="fetchSnapshots">↻ Rafraîchir</AppButton>
      <AppButton
        v-if="canManage"
        variant="secondary"
        :disabled="busy || !selectedGuildId"
        title="Importer une sauvegarde exportée (clonage cross-serveur)"
        @click="triggerImport"
      >
        ⬆ Importer
      </AppButton>
      <AppButton v-if="canManage" variant="primary" :disabled="busy || !selectedGuildId" @click="openCaptureModal">
        📸 Capturer maintenant
      </AppButton>
      <input
        ref="importInput"
        type="file"
        accept="application/json,.json"
        class="hidden-file"
        @change="onImportFile"
      />
    </template>

    <div v-if="!selectedGuildId" class="empty-state">
      <p>Sélectionnez un serveur dans la barre latérale pour gérer ses sauvegardes.</p>
    </div>

    <template v-else>
      <p v-if="!canManage" class="ro-banner">
        👁 Lecture seule — seuls le propriétaire du serveur et les super-admins peuvent
        capturer, restaurer, renommer ou supprimer des sauvegardes.
      </p>

      <p class="async-hint">
        ℹ️ Les captures et restaurations sont exécutées de façon asynchrone par le bot :
        l'action est lancée immédiatement mais son résultat apparaît après un court délai.
        Utilisez « Rafraîchir » pour mettre la liste à jour. (Suivi temps-réel non requis en v1.)
      </p>

      <LoadingState v-if="loading" />
      <ErrorState v-else-if="error" :message="error" @retry="fetchSnapshots" />

      <DataTable
        v-else
        :columns="columns"
        :rows="(snapshots as unknown as Record<string, unknown>[])"
        empty-message="Aucune sauvegarde pour ce serveur."
      >
        <template #cell-label="{ row }">
          <div v-if="editingId === (row as any).id" class="edit-inline">
            <input
              v-model="editLabel"
              class="input"
              type="text"
              @keyup.enter="saveEdit(row as unknown as SnapshotSummary)"
              @keyup.esc="cancelEdit"
            />
            <AppButton variant="success" size="sm" :disabled="busy" @click="saveEdit(row as unknown as SnapshotSummary)">✓</AppButton>
            <AppButton variant="secondary" size="sm" @click="cancelEdit">✕</AppButton>
          </div>
          <div v-else class="label-cell">
            <strong>{{ (row as any).label }}</strong>
            <button
              v-if="canManage"
              class="link-btn"
              title="Renommer"
              @click="startEdit(row as unknown as SnapshotSummary)"
            >✏️</button>
          </div>
        </template>

        <template #cell-created_at="{ value }">
          <span class="muted">{{ formatDateTime(value as string) }}</span>
        </template>

        <template #cell-role_count="{ value }">
          <span class="count-pill">{{ value }}</span>
        </template>
        <template #cell-channel_count="{ value }">
          <span class="count-pill">{{ value }}</span>
        </template>

        <template #cell-actions="{ row }">
          <div v-if="canManage" class="row-actions">
            <AppButton variant="secondary" size="sm" :disabled="busy" @click="openRestoreModal(row as unknown as SnapshotSummary)">
              ⟲ Restaurer
            </AppButton>
            <AppButton variant="secondary" size="sm" :disabled="busy" title="Exporter (JSON) — pour cloner sur un autre serveur" @click="exportSnapshot(row as unknown as SnapshotSummary)">
              ⬇
            </AppButton>
            <AppButton variant="danger" size="sm" :disabled="busy" @click="removeSnapshot(row as unknown as SnapshotSummary)">
              🗑
            </AppButton>
          </div>
          <span v-else class="muted small">—</span>
        </template>
      </DataTable>
    </template>

    <!-- ── Modale capture ── -->
    <div v-if="showCaptureModal" class="modal-backdrop" @click.self="showCaptureModal = false">
      <div class="modal-card">
        <h3>📸 Capturer une sauvegarde</h3>
        <p class="muted">
          Un instantané des rôles et salons du serveur va être capturé par le bot.
          Donnez-lui un nom (optionnel) pour le retrouver facilement.
        </p>
        <div class="modal-form">
          <label>
            Nom de la sauvegarde
            <input v-model="captureLabel" class="input" type="text" placeholder="Ex : avant refonte des salons" />
          </label>
        </div>
        <div class="modal-actions">
          <AppButton variant="secondary" :disabled="busy" @click="showCaptureModal = false">Annuler</AppButton>
          <AppButton variant="primary" :disabled="busy" @click="runCapture">
            {{ busy ? "Lancement…" : "📸 Lancer la capture" }}
          </AppButton>
        </div>
      </div>
    </div>

    <!-- ── Modale restauration ── -->
    <div v-if="showRestoreModal && restoreTarget" class="modal-backdrop" @click.self="closeRestoreModal">
      <div class="modal-card">
        <h3>⟲ Restaurer « {{ restoreTarget.label }} »</h3>
        <p class="muted">
          Les rôles et salons de cette sauvegarde ({{ restoreTarget.role_count }} rôles,
          {{ restoreTarget.channel_count }} salons) seront recréés sur le serveur.
          L'opération est exécutée de façon asynchrone par le bot.
        </p>

        <!-- Aperçu du contenu du snapshot -->
        <div class="preview-box">
          <div v-if="previewLoading" class="muted small">Chargement de l'aperçu…</div>
          <template v-else-if="preview">
            <div class="preview-cols">
              <div class="preview-col">
                <h4>Rôles ({{ preview.roles?.length ?? 0 }})</h4>
                <ul>
                  <li v-for="(r, i) in (preview.roles ?? []).slice(0, 12)" :key="i">{{ r.name }}</li>
                  <li v-if="(preview.roles?.length ?? 0) > 12" class="muted">
                    +{{ (preview.roles?.length ?? 0) - 12 }} autres…
                  </li>
                </ul>
              </div>
              <div class="preview-col">
                <h4>Salons ({{ preview.channels?.length ?? 0 }})</h4>
                <ul>
                  <li v-for="(c, i) in (preview.channels ?? []).slice(0, 12)" :key="i">
                    {{ c.name }}<span v-if="c.kind" class="muted small"> · {{ c.kind }}</span>
                  </li>
                  <li v-if="(preview.channels?.length ?? 0) > 12" class="muted">
                    +{{ (preview.channels?.length ?? 0) - 12 }} autres…
                  </li>
                </ul>
              </div>
            </div>
            <p class="muted small">
              Sans « wipe », la restauration RÉUTILISE les rôles/salons de même nom
              déjà présents (pas de doublon) et ne crée que ce qui manque.
            </p>
          </template>
        </div>

        <label class="checkbox danger-box">
          <input type="checkbox" v-model="restoreWipe" />
          <span>
            <strong>Vider le serveur d'abord (wipe)</strong> — supprime tous les rôles et
            salons existants avant de restaurer. <strong>Destructif et irréversible.</strong>
          </span>
        </label>

        <!-- Double confirmation TRES explicite pour le wipe -->
        <div v-if="restoreWipe" class="wipe-confirm">
          <p class="danger-text">
            ⚠️ ATTENTION : cette action supprimera <strong>définitivement</strong> tous les
            rôles et salons actuels du serveur « {{ guildName }} ». Il n'y a pas de retour arrière.
          </p>
          <label class="field">
            Pour confirmer, tapez le nom exact du serveur : <code>{{ guildName }}</code>
            <input v-model="wipeConfirmText" class="input" type="text" placeholder="Nom du serveur" />
          </label>
          <label class="checkbox">
            <input type="checkbox" v-model="restoreConfirmed" />
            <span>Je comprends que le serveur sera vidé et que cette action est irréversible.</span>
          </label>
        </div>

        <div class="modal-actions">
          <AppButton variant="secondary" :disabled="busy" @click="closeRestoreModal">Annuler</AppButton>
          <AppButton
            :variant="restoreWipe ? 'danger' : 'primary'"
            :disabled="busy || (restoreWipe && (!restoreConfirmed || !wipeTextMatches))"
            @click="runRestore"
          >
            {{ busy ? "Lancement…" : restoreWipe ? "🗑 Vider puis restaurer" : "⟲ Restaurer" }}
          </AppButton>
        </div>
      </div>
    </div>
  </AdminPageShell>
</template>

<style scoped>
.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-secondary);
  font-size: 15px;
}
.muted { color: var(--text-secondary); margin: 0; }
.muted.small, .small { font-size: 11px; }

.ro-banner {
  background: color-mix(in srgb, var(--warning, #e67e22) 10%, var(--bg-secondary));
  border-left: 3px solid var(--warning, #e67e22);
  padding: 10px 14px; border-radius: 4px;
  font-size: 13px; margin-bottom: 14px;
}
.async-hint {
  background: color-mix(in srgb, var(--accent) 6%, var(--bg-secondary));
  border-left: 3px solid var(--accent);
  padding: 10px 14px; border-radius: 4px;
  font-size: 13px; line-height: 1.5; margin-bottom: 18px;
}

.label-cell { display: flex; align-items: center; gap: 8px; }
.edit-inline { display: flex; align-items: center; gap: 6px; }
.link-btn {
  background: none; border: none; cursor: pointer; font-size: 0.85rem;
  opacity: 0.6; padding: 0;
}
.link-btn:hover { opacity: 1; }

.count-pill {
  display: inline-block; padding: 2px 10px; border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 15%, transparent);
  font-size: 0.8rem; font-weight: 600;
}
.row-actions { display: flex; gap: 6px; justify-content: flex-end; }
.hidden-file { display: none; }

.preview-box {
  margin: 14px 0;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--accent) 4%, transparent);
}
.preview-cols { display: flex; gap: 20px; flex-wrap: wrap; }
.preview-col { flex: 1; min-width: 160px; }
.preview-col h4 { margin: 0 0 6px; font-size: 12px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.4px; }
.preview-col ul { list-style: none; margin: 0; padding: 0; font-size: 13px; max-height: 180px; overflow-y: auto; }
.preview-col li { padding: 2px 0; }

.input {
  background: var(--bg, var(--bg-secondary)); color: var(--text, var(--text-primary));
  border: 1px solid var(--border); border-radius: 8px;
  padding: 8px 12px; font-size: 0.9rem; font-family: inherit;
  outline: none; width: 100%;
}
.input:focus { border-color: var(--accent); }

/* Modales */
.modal-backdrop {
  position: fixed; inset: 0; background: rgba(0,0,0,0.7);
  display: flex; align-items: center; justify-content: center;
  z-index: 1000; padding: 30px;
}
.modal-card {
  background: var(--bg-card, var(--surface)); border: 1px solid var(--border);
  border-radius: 14px; padding: 24px 28px; max-width: 520px; width: 100%;
}
.modal-card h3 { margin: 0 0 12px; font-size: 17px; }
.modal-form { display: flex; flex-direction: column; gap: 14px; margin: 18px 0; }
.modal-form label, .field {
  display: flex; flex-direction: column; gap: 6px;
  font-size: 12px; color: var(--text-secondary);
}
.field code {
  display: inline-block; font-family: "JetBrains Mono", monospace;
  color: var(--accent); font-size: 0.9em;
}
.modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 18px; }

.checkbox {
  display: flex; align-items: flex-start; gap: 10px;
  font-size: 13px; color: var(--text-primary, var(--text)); margin: 16px 0 0;
  line-height: 1.4;
}
.checkbox input { margin-top: 3px; flex-shrink: 0; }
.danger-box {
  background: color-mix(in srgb, var(--danger) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--danger) 40%, var(--border));
  border-radius: 8px; padding: 12px 14px;
}
.wipe-confirm {
  margin-top: 14px; padding: 14px;
  border: 1px solid var(--danger); border-radius: 8px;
  background: color-mix(in srgb, var(--danger) 6%, transparent);
  display: flex; flex-direction: column; gap: 12px;
}
.danger-text { color: var(--danger); font-size: 13px; margin: 0; line-height: 1.5; }
</style>
