<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

async function handleStartResize(e: MouseEvent) {
  if (e.button !== 0) return;

  try {
    await appWindow.startResizeDragging('SouthEast');
  } catch (err) {
    console.error('Failed to start resizing:', err);
  }
}
</script>

<template>
  <div
    class="fixed bottom-0 right-0 w-4 h-4 cursor-se-resize z-50 group"
    @mousedown="handleStartResize"
  >
    <!-- Fabric-style resize grip -->
    <svg
      class="w-full h-full text-stone-400/60 dark:text-stone-500/50 group-hover:text-stone-500/80 dark:group-hover:text-stone-400/70 transition-colors duration-200"
      viewBox="0 0 16 16"
      fill="currentColor"
    >
      <circle cx="12" cy="12" r="1.5" />
      <circle cx="8" cy="12" r="1.5" />
      <circle cx="12" cy="8" r="1.5" />
      <circle cx="4" cy="12" r="1.5" />
      <circle cx="8" cy="8" r="1.5" />
      <circle cx="12" cy="4" r="1.5" />
    </svg>
  </div>
</template>
