<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { UnlistenFn } from '@tauri-apps/api/event';

const appWindow = getCurrentWindow();
const isMaximized = ref(false);
let unlistenResize: UnlistenFn | null = null;

async function checkMaximized() {
  try {
    isMaximized.value = await appWindow.isMaximized();
  } catch (e) {
    console.error('Failed to check maximized state:', e);
  }
}

async function handleMinimize() {
  try {
    await appWindow.minimize();
  } catch (e) {
    console.error('Failed to minimize:', e);
  }
}

async function handleToggleMaximize() {
  try {
    await appWindow.toggleMaximize();
    await checkMaximized();
  } catch (e) {
    console.error('Failed to toggle maximize:', e);
  }
}

async function handleClose() {
  try {
    await appWindow.close();
  } catch (e) {
    console.error('Failed to close:', e);
  }
}

async function handleStartDrag(e: MouseEvent) {
  if (e.button !== 0) return;
  try {
    await appWindow.startDragging();
  } catch (e) {
    console.error('Failed to start dragging:', e);
  }
}

onMounted(async () => {
  await checkMaximized();
  try {
    unlistenResize = await appWindow.onResized(async () => {
      await checkMaximized();
    });
  } catch (e) {
    console.error('Failed to listen for resize:', e);
  }
});

onUnmounted(() => {
  if (unlistenResize) {
    unlistenResize();
  }
});
</script>

<template>
  <div
    class="h-11 bg-fabric-linen border-b border-fabric-sand/60 flex items-center justify-between select-none shrink-0"
    style="background-image: url('data:image/svg+xml,%3Csvg width=\'8\' height=\'8\' viewBox=\'0 0 8 8\' xmlns=\'http://www.w3.org/2000/svg\'%3E%3Cg fill=\'%238b7355\' fill-opacity=\'0.04\'%3E%3Cpath d=\'M0 0h4v4H0V0zm4 4h4v4H4V4z\'/%3E%3C/g%3E%3C/svg%3E')"
  >
    <!-- Left: App Logo & Title (draggable) -->
    <div
      class="flex items-center gap-3 px-4 flex-1 h-full cursor-default"
      @mousedown="handleStartDrag"
    >
      <!-- Fabric-style logo -->
      <div class="w-6 h-6 bg-primary-500/90 rounded-md flex items-center justify-center shadow-sm border border-primary-600/30 relative overflow-hidden">
        <!-- Stitch effect -->
        <div class="absolute inset-0.5 border border-dashed border-primary-300/30 rounded pointer-events-none"></div>
        <svg class="w-3.5 h-3.5 text-white/90 relative z-10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
      <div class="pointer-events-none">
        <span class="text-sm font-semibold text-fabric-sepia tracking-wide">Narrative Loom</span>
        <span class="text-xs text-fabric-thread/60 ml-2 font-normal">织语工坊</span>
      </div>
    </div>

    <!-- Right: Window Controls -->
    <div class="flex items-center h-full">
      <!-- Settings -->
      <router-link
        to="/settings"
        class="h-full w-12 flex items-center justify-center transition-all duration-180 hover:bg-fabric-sand/40 active:bg-fabric-sand/60 group"
        title="设置"
      >
        <svg class="w-4 h-4 text-fabric-thread/70 group-hover:text-fabric-sepia transition-colors duration-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      </router-link>

      <!-- Divider -->
      <div class="w-px h-4 bg-fabric-sand/50"></div>

      <!-- Minimize -->
      <button
        @click="handleMinimize"
        class="h-full w-12 flex items-center justify-center transition-all duration-180 hover:bg-fabric-sand/40 active:bg-fabric-sand/60 group"
        title="最小化"
      >
        <svg class="w-3.5 h-3.5 text-fabric-thread/70 group-hover:text-fabric-sepia transition-colors duration-180" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M5 12h14" stroke-linecap="round" />
        </svg>
      </button>

      <!-- Maximize/Restore -->
      <button
        @click="handleToggleMaximize"
        class="h-full w-12 flex items-center justify-center transition-all duration-180 hover:bg-fabric-sand/40 active:bg-fabric-sand/60 group"
        :title="isMaximized ? '还原' : '最大化'"
      >
        <svg v-if="isMaximized" class="w-3 h-3 text-fabric-thread/70 group-hover:text-fabric-sepia transition-colors duration-180" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="5" y="9" width="10" height="10" rx="1" />
          <path d="M9 9V5a1 1 0 011-1h9a1 1 0 011 1v9a1 1 0 01-1 1h-4" />
        </svg>
        <svg v-else class="w-3 h-3 text-fabric-thread/70 group-hover:text-fabric-sepia transition-colors duration-180" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="4" y="4" width="16" height="16" rx="1" />
        </svg>
      </button>

      <!-- Close -->
      <button
        @click="handleClose"
        class="h-full w-12 flex items-center justify-center transition-all duration-180 hover:bg-red-400/80 active:bg-red-500/90 group"
        title="关闭"
      >
        <svg class="w-3.5 h-3.5 text-fabric-thread/70 group-hover:text-white transition-colors duration-180" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M6 6l12 12M6 18L18 6" stroke-linecap="round" />
        </svg>
      </button>
    </div>
  </div>
</template>
