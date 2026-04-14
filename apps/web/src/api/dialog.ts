// Ouverture de fichier via <input type="file">. En web, on utilise <input type="file">
// via un helper async qui resout le chemin en File blob.

export interface OpenDialogOptions {
  multiple?: boolean;
  directory?: boolean;
  filters?: Array<{ name: string; extensions: string[] }>;
}

export async function open(options: OpenDialogOptions = {}): Promise<string | string[] | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = !!options.multiple;
    if (options.filters && options.filters.length) {
      input.accept = options.filters.flatMap((f) => f.extensions.map((e) => `.${e}`)).join(",");
    }
    input.onchange = () => {
      const files = Array.from(input.files ?? []);
      if (!files.length) { resolve(null); return; }
      const names = files.map((f) => f.name);
      resolve(options.multiple ? names : names[0]);
    };
    input.click();
  });
}

export async function save(_opts: unknown = {}): Promise<string | null> { return null; }
export async function confirm(msg: string): Promise<boolean> { return window.confirm(msg); }
export async function message(msg: string): Promise<void> { window.alert(msg); }
export async function ask(msg: string): Promise<boolean> { return window.confirm(msg); }
