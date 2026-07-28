<script setup lang="ts">
import AppInput from "@/components/atoms/AppInput.vue";
import { ref, watch } from "vue";
import { discordRolesService } from "@/services/discordRolesService";
import { useDiscordRoles } from "../../composables/useDiscordRoles";
import { useGuildSelector } from "../../composables/useGuildSelector";
import type { DiscordRole } from "../../types";
import AppModal from "../atoms/AppModal.vue";
import AppButton from "../atoms/AppButton.vue";
import { useToast } from "../../composables/useToast";
import { errMsg } from "@/utils/errMsg";

const { error: showError } = useToast();

const props = defineProps<{
  /** null = closed, sinon edition de ce role */
  target: DiscordRole | null;
}>();
const emit = defineEmits<{ close: [] }>();

const { selectedGuildId } = useGuildSelector();
const { fetchRoles } = useDiscordRoles();

const PERMISSION_FLAGS: { key: string; label: string; bit: bigint }[] = [
  { key: "admin", label: "Administrateur", bit: 0x8n },
  { key: "manage_guild", label: "Gerer le serveur", bit: 0x20n },
  { key: "manage_roles", label: "Gerer les roles", bit: 0x10000000n },
  { key: "manage_channels", label: "Gerer les salons", bit: 0x10n },
  { key: "kick", label: "Expulser", bit: 0x2n },
  { key: "ban", label: "Bannir", bit: 0x4n },
  { key: "moderate_members", label: "Moderer les membres", bit: 0x10000000000n },
  { key: "manage_messages", label: "Gerer les messages", bit: 0x2000n },
  { key: "mention_everyone", label: "Mentionner @everyone", bit: 0x20000n },
  { key: "send_messages", label: "Envoyer des messages", bit: 0x800n },
  { key: "connect", label: "Se connecter (vocal)", bit: 0x100000n },
  { key: "speak", label: "Parler (vocal)", bit: 0x200000n },
  { key: "mute_members", label: "Muter des membres", bit: 0x400000n },
  { key: "move_members", label: "Deplacer des membres", bit: 0x1000000n },
];

function parsePerms(permsStr: string): Record<string, boolean> {
  const bits = BigInt(permsStr);
  const result: Record<string, boolean> = {};
  for (const flag of PERMISSION_FLAGS) {
    result[flag.key] = (bits & flag.bit) !== 0n;
  }
  return result;
}

function buildPermsString(perms: Record<string, boolean>): string {
  let bits = 0n;
  for (const flag of PERMISSION_FLAGS) {
    if (perms[flag.key]) bits |= flag.bit;
  }
  return bits.toString();
}

const editName = ref("");
const editColor = ref("#000000");
const editMentionable = ref(false);
const editHoist = ref(false);
const editPerms = ref<Record<string, boolean>>({});
const saving = ref(false);

watch(
  () => props.target,
  (role) => {
    if (!role) return;
    editName.value = role.name;
    editColor.value = `#${role.color.toString(16).padStart(6, "0")}`;
    editMentionable.value = role.mentionable;
    editHoist.value = false; // Discord API ne retourne pas hoist dans notre sync
    editPerms.value = parsePerms(role.permissions);
  },
);

async function saveEdit() {
  if (!selectedGuildId.value || !props.target) return;
  saving.value = true;
  try {
    const colorInt = parseInt(editColor.value.replace("#", ""), 16);
    const permsStr = buildPermsString(editPerms.value);
    await discordRolesService.edit(selectedGuildId.value, props.target.id, {
      name: editName.value.trim() || null,
      color: colorInt,
      permissions: permsStr,
      mentionable: editMentionable.value,
      hoist: editHoist.value,
    });
    emit("close");
    await fetchRoles();
  } catch (e) {
    showError(`Erreur modification rôle : ${errMsg(e)}`);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <AppModal
    :visible="!!target"
    title="Modifier le role"
    size="md"
    @close="emit('close')"
  >
    <div class="edit-section">
      <span class="edit-section-title">Apparence</span>
      <div class="edit-grid-2">
        <div class="edit-field">
          <label>Nom</label>
          <AppInput v-model="editName" type="text" class="modal-input" placeholder="Nom du role" />
        </div>
        <div class="edit-field">
          <label>Couleur</label>
          <div class="color-row">
            <input v-model="editColor" type="color" class="color-picker-lg" />
            <span class="color-hex">{{ editColor }}</span>
          </div>
        </div>
      </div>
    </div>

    <div class="edit-section">
      <span class="edit-section-title">Options</span>
      <div class="options-row">
        <label class="option-toggle" :class="{ active: editMentionable }">
          <input v-model="editMentionable" type="checkbox" />
          <span class="option-label">Mentionnable</span>
          <span class="option-desc">Les membres peuvent mentionner ce role</span>
        </label>
        <label class="option-toggle" :class="{ active: editHoist }">
          <input v-model="editHoist" type="checkbox" />
          <span class="option-label">Afficher separement</span>
          <span class="option-desc">Separe les membres dans la sidebar</span>
        </label>
      </div>
    </div>

    <div class="edit-section">
      <span class="edit-section-title">Permissions</span>
      <div class="perms-grid">
        <label
          v-for="flag in PERMISSION_FLAGS"
          :key="flag.key"
          class="perm-chip"
          :class="{ active: editPerms[flag.key] }"
        >
          <input v-model="editPerms[flag.key]" type="checkbox" class="perm-cb" />
          <span>{{ flag.label }}</span>
        </label>
      </div>
    </div>

    <template #footer>
      <AppButton variant="secondary" @click="emit('close')">Annuler</AppButton>
      <AppButton variant="primary" :disabled="saving" @click="saveEdit">
        {{ saving ? 'Sauvegarde...' : 'Sauvegarder' }}
      </AppButton>
    </template>
  </AppModal>
</template>

<style scoped>
.edit-section { margin-bottom: 24px; }
.edit-section:last-child { margin-bottom: 0; }
.edit-section-title {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  display: block;
  margin-bottom: 12px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border);
}

.edit-grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
.edit-field label {
  display: block;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.modal-input {
  width: 100%;
  padding: 8px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: 13px;
  box-sizing: border-box;
}
.modal-input:focus {
  border-color: var(--accent);
  outline: none;
  box-shadow: var(--focus-ring);
}

.color-picker-lg {
  width: 44px;
  height: 44px;
  border: 2px solid var(--border);
  border-radius: 8px;
  cursor: pointer;
  padding: 0;
}
.color-row { display: flex; align-items: center; gap: 12px; }
.color-hex {
  font-size: 14px;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-secondary);
}

/* Options toggles */
.options-row { display: flex; gap: 12px; }
.option-toggle {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  cursor: pointer;
  padding: 12px 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-primary);
  transition: all var(--transition-fast);
}
.option-toggle:hover { border-color: var(--accent); }
.option-toggle.active { border-color: var(--accent); background: rgba(88,101,242,0.06); }
.option-toggle input { display: none; }
.option-label { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.option-desc { font-size: 11px; color: var(--text-secondary); }

/* Permissions grid */
.perms-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; }
.perm-chip {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-primary);
  font-size: 13px;
  color: var(--text-secondary);
  transition: all var(--transition-fast);
  user-select: none;
}
.perm-chip:hover { border-color: rgba(88,101,242,0.3); background: var(--bg-hover); }
.perm-chip.active {
  color: var(--accent);
  background: rgba(88,101,242,0.08);
  border-color: rgba(88,101,242,0.3);
  font-weight: 600;
}
.perm-cb { accent-color: var(--accent); width: 16px; height: 16px; }
</style>
