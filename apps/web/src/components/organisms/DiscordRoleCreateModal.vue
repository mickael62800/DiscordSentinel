<script setup lang="ts">
import AppInput from "@/components/atoms/AppInput.vue";
import { ref, watch } from "vue";
import { discordRolesService } from "@/services/discordRolesService";
import { useDiscordRoles } from "../../composables/useDiscordRoles";
import { useGuildSelector } from "../../composables/useGuildSelector";
import AppModal from "../atoms/AppModal.vue";
import AppButton from "../atoms/AppButton.vue";

const props = defineProps<{ visible: boolean }>();
const emit = defineEmits<{ close: [] }>();

const { selectedGuildId } = useGuildSelector();
const { fetchRoles } = useDiscordRoles();

const newRoleName = ref("");
const newRoleColor = ref("#5865F2");
const creating = ref(false);

watch(
  () => props.visible,
  (v) => {
    if (v) {
      newRoleName.value = "";
      newRoleColor.value = "#5865F2";
    }
  },
);

async function createRole() {
  if (!selectedGuildId.value || !newRoleName.value.trim()) return;
  creating.value = true;
  try {
    const colorInt = parseInt(newRoleColor.value.replace("#", ""), 16);
    await discordRolesService.create(selectedGuildId.value, {
      name: newRoleName.value.trim(),
      color: colorInt,
      permissions: null,
    });
    emit("close");
    await fetchRoles();
  } catch (e) {
    alert("Erreur creation role: " + e);
  } finally {
    creating.value = false;
  }
}
</script>

<template>
  <AppModal
    :visible="visible"
    title="Creer un role"
    size="sm"
    @close="emit('close')"
  >
    <div class="modal-field">
      <label>Nom</label>
      <AppInput v-model="newRoleName" type="text" class="modal-input" placeholder="Nom du role" />
    </div>
    <div class="modal-field">
      <label>Couleur</label>
      <div class="color-row">
        <input v-model="newRoleColor" type="color" class="color-picker" />
        <span class="color-hex">{{ newRoleColor }}</span>
      </div>
    </div>

    <template #footer>
      <AppButton variant="secondary" @click="emit('close')">Annuler</AppButton>
      <AppButton variant="primary" :disabled="!newRoleName.trim() || creating" @click="createRole">
        {{ creating ? 'Creation...' : 'Creer' }}
      </AppButton>
    </template>
  </AppModal>
</template>

<style scoped>
.modal-field { margin-bottom: 16px; }
.modal-field label {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 8px;
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

.color-picker {
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  padding: 0;
}
.color-row { display: flex; align-items: center; gap: 12px; }
.color-hex {
  font-size: 14px;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-secondary);
}
</style>
