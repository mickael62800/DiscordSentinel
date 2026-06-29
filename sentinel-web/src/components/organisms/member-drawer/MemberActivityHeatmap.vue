<script setup lang="ts">
import { computed } from "vue";
import type { UserActivity } from "../../../types";
import { heatmapData, heatColor } from "../../../utils/memberActivity";

const props = defineProps<{ activity: UserActivity[] }>();

const heat = computed(() => heatmapData(props.activity));
</script>

<template>
  <div v-if="heat.max > 0" class="section">
    <h3>🗓️ Heatmap activité (messages par heure)</h3>
    <div class="heatmap-wrap">
      <table class="watch-heatmap">
        <thead>
          <tr>
            <th></th>
            <th v-for="h in 24" :key="h" class="hm-hour">{{ h - 1 }}h</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(dn, di) in heat.days" :key="di">
            <td class="hm-day">{{ dn }}</td>
            <td
              v-for="hi in 24"
              :key="hi"
              class="hm-cell"
              :style="{ backgroundColor: heatColor(heat.grid[di][hi - 1], heat.max) }"
              :title="`${dn} ${hi - 1}h : ${heat.grid[di][hi - 1]} msg`"
            ></td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.section { margin-bottom: 20px; }
.section h3 { margin: 0 0 10px 0; font-size: 14px; font-weight: 600; }

.heatmap-wrap { width: 100%; overflow-x: auto; }
.watch-heatmap {
  border-collapse: separate;
  border-spacing: 2px;
  width: 100%;
  min-width: 540px;
  table-layout: fixed;
}
.hm-hour { font-size: 9px; color: var(--text-secondary); padding: 1px 0; text-align: center; }
.hm-day {
  font-size: 11px;
  color: var(--text-secondary);
  padding-right: 6px;
  white-space: nowrap;
  text-align: right;
  width: 36px;
}
.hm-cell { height: 22px; border-radius: 3px; cursor: default; }
</style>
