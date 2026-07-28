import { storeToRefs } from "pinia";
import { useAuthStore } from "@/stores/authStore";

export function useAuth() {
  const store = useAuthStore();
  const { user, loading, error, initialized } = storeToRefs(store);

  return {
    user, loading, error, initialized,
    checkSession: store.checkSession,
    login: store.login,
    logout: store.logout,
    avatarUrl: store.avatarUrl,
  };
}
