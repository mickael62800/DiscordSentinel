import { storeToRefs } from "pinia";
import { useAuthStore } from "@/stores/authStore";

export function useAuth() {
  const store = useAuthStore();
  const { user, loading, error, initialized, hasConfig } = storeToRefs(store);

  return {
    user, loading, error, initialized, hasConfig,
    checkSession: store.checkSession,
    saveConfig: store.saveConfig,
    clearConfig: store.clearConfig,
    login: store.login,
    logout: store.logout,
    avatarUrl: store.avatarUrl,
  };
}
