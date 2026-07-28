<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import RoleSelect from "@/components/atoms/RoleSelect.vue";
import {
  gamePortalService,
  type GameTemplate,
} from "@/services/gamePortalService";
import { useToast } from "@/composables/useToast";

const props = defineProps<{
  templates: GameTemplate[];
  guildId: string | null;
}>();

const { success, error: showError } = useToast();
// slug -> roleId ("" = aucun)
const roles = ref<Record<string, string>>({});

async function load() {
  if (!props.guildId) return;
  const settings = await gamePortalService
    .listTemplateSettings(props.guildId)
    .catch(() => []);
  const map: Record<string, string> = {};
  for (const s of settings) map[s.template_slug] = s.discord_role_id ?? "";
  roles.value = map;
}

async function onChange(slug: string, roleId: string) {
  if (!props.guildId) return;
  roles.value = { ...roles.value, [slug]: roleId };
  try {
    await gamePortalService.setTemplateRole(props.guildId, slug, roleId || null);
    success("Rôle enregistré.");
  } catch {
    showError("Erreur lors de l'enregistrement du rôle.");
  }
}

onMounted(load);
watch(() => props.guildId, load);
</script>

<template>
  <section class="card roles-panel">
    <h3>🔔 Rôle Discord par jeu</h3>
    <p class="hint">
      À l'ouverture d'un serveur, ce rôle est mentionné et devient le seul à voir
      les salons privés de la session (texte + vocal).
    </p>
    <div class="rows">
      <div v-for="t in templates" :key="t.slug" class="row">
        <span class="game">
          <span v-if="t.icon" class="icon">{{ t.icon }}</span>
          {{ t.name }}
        </span>
        <RoleSelect
          :model-value="roles[t.slug] ?? ''"
          :guild-id="guildId"
          @update:model-value="(v: string) => onChange(t.slug, v)"
        />
      </div>
      <p v-if="templates.length === 0" class="hint">Aucun jeu dans le catalogue.</p>
    </div>
  </section>
</template>

<style scoped>
.roles-panel h3 { margin: 0 0 4px; }
.hint { color: var(--text-muted, #9ca3af); font-size: 0.85rem; margin: 0 0 12px; }
.rows { display: flex; flex-direction: column; gap: 10px; }
.row {
  display: flex; align-items: center; justify-content: space-between; gap: 12px;
}
.game { font-weight: 600; display: flex; align-items: center; gap: 6px; }
.row :deep(select) { max-width: 260px; }
</style>
