import { open as tauriOpen } from "@tauri-apps/plugin-dialog";

const browserPreview = typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);

export async function open(options: { directory?: boolean; multiple?: boolean; [key: string]: unknown }): Promise<string | string[] | null> {
  if (!browserPreview) return tauriOpen(options as Parameters<typeof tauriOpen>[0]);
  const path = options.directory ? "/preview/legacy-archives" : "/preview/sample-asset.zip";
  return options.multiple ? [path] : path;
}
