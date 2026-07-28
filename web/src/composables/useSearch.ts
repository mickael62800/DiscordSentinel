import { ref, computed, type Ref, type ComputedRef } from "vue";

export function useSearch<T>(
  items: Ref<T[]> | ComputedRef<T[]>,
  fields: (keyof T | ((item: T) => string | null | undefined))[],
): {
  search: Ref<string>;
  filtered: ComputedRef<T[]>;
} {
  const search = ref("");

  const filtered = computed(() => {
    const q = search.value.trim().toLowerCase();
    if (!q) return items.value;

    return items.value.filter((item) =>
      fields.some((field) => {
        const value = typeof field === "function"
          ? field(item)
          : String(item[field] ?? "");
        return value?.toLowerCase().includes(q);
      }),
    );
  });

  return { search, filtered };
}
