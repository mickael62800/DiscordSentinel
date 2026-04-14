import { ref } from "vue";

const visible = ref(false);
const title = ref("Confirmation");
const message = ref("");
let _resolve: ((value: boolean) => void) | null = null;

export function useConfirm() {
  function confirm(opts: { title?: string; message: string }): Promise<boolean> {
    title.value = opts.title ?? "Confirmation";
    message.value = opts.message;
    visible.value = true;

    return new Promise<boolean>((resolve) => {
      _resolve = resolve;
    });
  }

  function resolve(value: boolean) {
    visible.value = false;
    if (_resolve) {
      _resolve(value);
      _resolve = null;
    }
  }

  return { visible, title, message, confirm, resolve };
}
