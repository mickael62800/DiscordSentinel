import { ref, computed, watch, type Ref, type ComputedRef } from "vue";

export function usePagination<T>(items: Ref<T[]> | ComputedRef<T[]>, defaultPerPage = 25) {
  const currentPage = ref(1);
  const perPage = ref(defaultPerPage);

  const totalItems = computed(() => items.value.length);
  const totalPages = computed(() => Math.max(1, Math.ceil(totalItems.value / perPage.value)));

  const paginatedItems = computed(() => {
    const start = (currentPage.value - 1) * perPage.value;
    return items.value.slice(start, start + perPage.value);
  });

  // Reset page quand les filtres changent ou le nombre d'items change
  watch([totalItems, perPage], () => {
    if (currentPage.value > totalPages.value) {
      currentPage.value = 1;
    }
  });

  function goToPage(page: number) {
    if (page >= 1 && page <= totalPages.value) {
      currentPage.value = page;
    }
  }

  function nextPage() {
    goToPage(currentPage.value + 1);
  }

  function prevPage() {
    goToPage(currentPage.value - 1);
  }

  return {
    currentPage,
    perPage,
    totalItems,
    totalPages,
    paginatedItems,
    goToPage,
    nextPage,
    prevPage,
  };
}
