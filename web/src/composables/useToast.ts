import { ref } from "vue";

export type ToastType = "success" | "error" | "warning" | "info";

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
  duration: number;
}

const toasts = ref<Toast[]>([]);
let nextId = 0;

export function useToast() {
  function show(type: ToastType, message: string, duration = 4000) {
    const id = nextId++;
    toasts.value.push({ id, type, message, duration });

    if (duration > 0) {
      setTimeout(() => remove(id), duration);
    }
  }

  function remove(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }

  function success(message: string) {
    show("success", message);
  }

  function error(message: string) {
    show("error", message, 6000);
  }

  function warning(message: string) {
    show("warning", message, 5000);
  }

  function info(message: string) {
    show("info", message);
  }

  return { toasts, show, remove, success, error, warning, info };
}
