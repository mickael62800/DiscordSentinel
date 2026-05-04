<script setup lang="ts">
import AppSelect from "@/components/atoms/AppSelect.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import { computed, onMounted, ref, watch } from "vue";
import { invitationsService, type InvitationDto } from "@/services/invitationsService";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";

const { selectedGuildId } = useGuildSelector();
const { success, error: showError } = useToast();

const invitations = ref<InvitationDto[]>([]);
const loading = ref(false);
const filterStatus = ref<"all" | "active" | "used" | "expired">("active");

// Form generation
const showForm = ref(false);
const formRole = ref<"viewer" | "moderator" | "admin" | "owner">("viewer");
const formExpiresHours = ref(168); // 7 jours
const formNotes = ref("");
const generating = ref(false);

// Modal "code généré" pour copy
const generatedCode = ref<InvitationDto | null>(null);

async function load() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  try {
    invitations.value = await invitationsService.list(selectedGuildId.value);
  } catch (e: any) {
    showError(`Echec chargement invitations : ${e?.message ?? e}`);
  } finally {
    loading.value = false;
  }
}

const filtered = computed(() => {
  if (filterStatus.value === "all") return invitations.value;
  return invitations.value.filter((inv) => inv.status === filterStatus.value);
});

const counts = computed(() => ({
  active: invitations.value.filter((i) => i.status === "active").length,
  used: invitations.value.filter((i) => i.status === "used").length,
  expired: invitations.value.filter((i) => i.status === "expired").length,
  total: invitations.value.length,
}));

async function generate() {
  if (!selectedGuildId.value) return;
  generating.value = true;
  try {
    const inv = await invitationsService.create({
      guild_id: selectedGuildId.value,
      role: formRole.value,
      expires_in_hours: formExpiresHours.value,
      notes: formNotes.value.trim() || undefined,
    });
    generatedCode.value = inv;
    showForm.value = false;
    formNotes.value = "";
    formRole.value = "viewer";
    formExpiresHours.value = 168;
    await load();
    success("Code d'invitation généré.");
  } catch (e: any) {
    showError(`Echec génération : ${e?.message ?? e}`);
  } finally {
    generating.value = false;
  }
}

async function revoke(code: string) {
  if (!confirm(`Révoquer le code ${code} ? Il ne sera plus utilisable.`)) return;
  try {
    await invitationsService.revoke(code);
    success("Code révoqué.");
    await load();
  } catch (e: any) {
    showError(`Echec révocation : ${e?.message ?? e}`);
  }
}

async function copyCode(code: string) {
  try {
    await navigator.clipboard.writeText(code);
    success("Code copié dans le presse-papier.");
  } catch {
    showError("Impossible de copier (utilise sélection manuelle).");
  }
}

function fmtDate(s: string | null): string {
  if (!s) return "—";
  return new Date(s).toLocaleString("fr-FR");
}

function statusLabel(s: string): string {
  return { active: "Actif", used: "Utilisé", expired: "Expiré" }[s] ?? s;
}

function roleLabel(r: string): string {
  return { viewer: "Viewer", moderator: "Modérateur", admin: "Admin", owner: "Owner" }[r] ?? r;
}

onMounted(load);
watch(selectedGuildId, load);
</script>

<template>
  <section class="inv-section">
    <div class="inv-head">
      <div>
        <h3>🎟️ Codes d'invitation</h3>
        <p class="muted">
          Génère un code à usage unique pour inviter quelqu'un. Le code attribue
          automatiquement le rôle choisi quand l'invité se connecte avec son compte Discord.
        </p>
      </div>
      <button class="btn primary" @click="showForm = !showForm">
        {{ showForm ? "✕ Annuler" : "+ Générer un code" }}
      </button>
    </div>

    <!-- Form génération -->
    <div v-if="showForm" class="form-card">
      <div class="form-row">
        <label>
          Rôle attribué
          <AppSelect v-model="formRole">
            <option value="viewer">Viewer (lecture seule)</option>
            <option value="moderator">Modérateur</option>
            <option value="admin">Admin</option>
            <option value="owner">Owner</option>
          </AppSelect>
        </label>
        <label>
          Expiration
          <AppSelect v-model.number="formExpiresHours">
            <option :value="24">1 jour</option>
            <option :value="168">7 jours</option>
            <option :value="720">30 jours</option>
            <option :value="0">Jamais</option>
          </AppSelect>
        </label>
        <label class="grow">
          Notes (privé, ex: "Pour Bob, modo backup")
          <AppInput v-model="formNotes" placeholder="(facultatif)" />
        </label>
      </div>
      <div class="form-actions">
        <button class="btn primary" :disabled="generating" @click="generate">
          {{ generating ? "Génération…" : "🎫 Générer le code" }}
        </button>
      </div>
    </div>

    <!-- Modal code généré -->
    <div v-if="generatedCode" class="generated-modal" @click.self="generatedCode = null">
      <div class="generated-card">
        <h4>✅ Code d'invitation généré</h4>
        <div class="big-code">{{ generatedCode.code }}</div>
        <p class="muted small">
          <strong>Rôle :</strong> {{ roleLabel(generatedCode.role) }} ·
          <strong>Expire :</strong> {{ fmtDate(generatedCode.expires_at) }}
        </p>
        <p class="hint">
          Partage ce code à l'invité (DM Discord, message privé, etc.). Il pourra l'utiliser
          UNE SEULE FOIS sur la page de connexion. Le code disparaîtra après la première
          utilisation.
        </p>
        <div class="generated-actions">
          <button class="btn primary" @click="copyCode(generatedCode.code)">📋 Copier</button>
          <button class="btn" @click="generatedCode = null">Fermer</button>
        </div>
      </div>
    </div>

    <!-- Filtres + compteurs -->
    <div class="filter-bar">
      <AppSelect v-model="filterStatus">
        <option value="all">Tous ({{ counts.total }})</option>
        <option value="active">Actifs ({{ counts.active }})</option>
        <option value="used">Utilisés ({{ counts.used }})</option>
        <option value="expired">Expirés ({{ counts.expired }})</option>
      </AppSelect>
      <button class="btn xs" @click="load">↻</button>
    </div>

    <!-- Table -->
    <div v-if="loading" class="muted">Chargement…</div>
    <div v-else-if="filtered.length === 0" class="empty">
      Aucun code d'invitation pour ce filtre.
    </div>
    <table v-else class="inv-table">
      <thead>
        <tr>
          <th>Code</th>
          <th>Rôle</th>
          <th>Statut</th>
          <th>Créé le</th>
          <th>Expire</th>
          <th>Utilisé par</th>
          <th>Notes</th>
          <th class="actions-h">Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="inv in filtered" :key="inv.code" :class="`row-${inv.status}`">
          <td><code class="code-cell">{{ inv.code }}</code></td>
          <td><span class="role-pill">{{ roleLabel(inv.role) }}</span></td>
          <td><span class="status-pill" :class="inv.status">{{ statusLabel(inv.status) }}</span></td>
          <td class="small muted">{{ fmtDate(inv.created_at) }}</td>
          <td class="small">{{ fmtDate(inv.expires_at) }}</td>
          <td class="small mono">{{ inv.used_by_discord_id ?? "—" }}</td>
          <td class="small muted notes">{{ inv.notes ?? "—" }}</td>
          <td class="actions">
            <button v-if="inv.status === 'active'" class="btn xs" @click="copyCode(inv.code)">📋</button>
            <button v-if="inv.status === 'active'" class="btn xs danger" @click="revoke(inv.code)">🗑</button>
            <span v-else class="muted">—</span>
          </td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
.inv-section {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 18px 20px;
  margin-top: 20px;
}
.inv-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 16px;
  margin-bottom: 16px;
}
.inv-head h3 { margin: 0 0 4px; font-size: 15px; }
.muted { color: var(--text-secondary); font-size: 12px; margin: 0; }
.muted.small { font-size: 11px; }
.small { font-size: 11px; }
.mono { font-family: "JetBrains Mono", monospace; }

.form-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px;
  margin-bottom: 16px;
}
.form-row {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}
.form-row label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.4px;
}
.form-row label.grow { flex: 1; min-width: 200px; }
.form-row input, .form-row select {
  padding: 7px 10px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
}
.form-actions { display: flex; justify-content: flex-end; }

.generated-modal {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 30px;
}
.generated-card {
  background: var(--bg-card);
  border: 2px solid var(--accent);
  border-radius: 14px;
  padding: 24px 28px;
  max-width: 500px;
  width: 100%;
  text-align: center;
}
.generated-card h4 { margin: 0 0 16px; font-size: 18px; }
.big-code {
  font-family: "JetBrains Mono", monospace;
  font-size: 28px;
  font-weight: 700;
  letter-spacing: 4px;
  background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 25%, var(--bg-secondary)), var(--bg-secondary));
  padding: 18px 24px;
  border-radius: 10px;
  margin: 16px 0;
  color: var(--accent);
  user-select: all;
}
.hint {
  font-size: 12px;
  color: var(--text-secondary);
  margin: 12px 0 18px;
  line-height: 1.5;
}
.generated-actions {
  display: flex;
  justify-content: center;
  gap: 10px;
}

.filter-bar {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 12px;
}
.filter-bar select {
  padding: 5px 10px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
}

.inv-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.inv-table th, .inv-table td {
  padding: 8px 10px;
  text-align: left;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
}
.inv-table th {
  font-size: 10px;
  text-transform: uppercase;
  color: var(--text-secondary);
  letter-spacing: 0.4px;
}
.inv-table .actions-h { text-align: right; }
.inv-table .actions { text-align: right; white-space: nowrap; }
.inv-table .actions .btn { margin-left: 4px; }
.inv-table tr.row-used { opacity: 0.55; }
.inv-table tr.row-expired { opacity: 0.5; }
.inv-table .notes { max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.code-cell {
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
  font-weight: 700;
  background: var(--bg-secondary);
  padding: 3px 8px;
  border-radius: 4px;
  letter-spacing: 1px;
}

.role-pill {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 10px;
  text-transform: uppercase;
  font-weight: 700;
  letter-spacing: 0.4px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
}

.status-pill {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.4px;
}
.status-pill.active { background: color-mix(in srgb, var(--success, #2ecc71) 18%, transparent); color: var(--success, #2ecc71); }
.status-pill.used { background: color-mix(in srgb, var(--accent) 18%, transparent); color: var(--accent); }
.status-pill.expired { background: color-mix(in srgb, var(--danger) 14%, transparent); color: var(--danger); }

.btn {
  padding: 7px 14px;
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
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn.xs { padding: 3px 8px; font-size: 11px; }
.btn.primary { background: var(--accent); color: white; border-color: var(--accent); }
.btn.primary:hover:not(:disabled) { filter: brightness(1.1); color: white; }
.btn.danger { border-color: color-mix(in srgb, var(--danger) 50%, var(--border)); color: var(--danger); }
.btn.danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 15%, var(--bg-secondary)); }

.empty {
  padding: 20px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 12px;
  font-style: italic;
}
</style>
