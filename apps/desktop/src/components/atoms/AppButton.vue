<script setup lang="ts">
defineProps<{
  variant?: "primary" | "secondary";
}>();
</script>

<template>
  <button :class="variant ?? 'primary'">
    <slot />
  </button>
</template>
