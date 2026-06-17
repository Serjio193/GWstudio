import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export async function safeInvoke(command, args, fallback) {
  try {
    return await invoke(command, args);
  } catch (error) {
    if (typeof fallback === "function") {
      return fallback(error);
    }
    throw error;
  }
}

export { listen };
export { getCurrentWindow };
