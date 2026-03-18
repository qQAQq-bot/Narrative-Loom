import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export function useTauri() {
  const invokeCommand = async <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
    try {
      return await invoke<T>(command, args);
    } catch (error) {
      console.error(`Tauri command '${command}' failed:`, error);
      throw error;
    }
  };

  const listenEvent = async <T>(
    event: string,
    handler: (payload: T) => void
  ): Promise<UnlistenFn> => {
    return await listen<T>(event, (e) => {
      handler(e.payload);
    });
  };

  return {
    invoke: invokeCommand,
    listen: listenEvent,
  };
}
