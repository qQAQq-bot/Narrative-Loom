<script setup lang="ts">
import { computed } from 'vue';

interface BatchProgress {
  currentIndex: number;
  totalCount: number;
  currentChapterTitle: string;
  currentAgentType: string;
  isRunning: boolean;
  completedChapters: string[];
  failedChapters: { id: string; title: string; error: string }[];
}

const props = defineProps<{
  show: boolean;
  progress: BatchProgress;
  selectedChapters: { id: string; title: string }[];
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'cancel'): void;
  (e: 'start'): void;
}>();

const progressPercent = computed(() => {
  if (props.progress.totalCount === 0) return 0;
  return Math.round((props.progress.currentIndex / props.progress.totalCount) * 100);
});

const statusText = computed(() => {
  if (!props.progress.isRunning) {
    if (props.progress.currentIndex === props.progress.totalCount && props.progress.totalCount > 0) {
      return '批量分析完成';
    }
    return '准备开始分析';
  }
  return `正在分析第 ${props.progress.currentIndex + 1} 章 / 共 ${props.progress.totalCount} 章`;
});

// Only allow closing when not running
function handleBackdropClick() {
  if (!props.progress.isRunning) {
    emit('close');
  }
}

function handleCloseClick() {
  if (!props.progress.isRunning) {
    emit('close');
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="show"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50"
        @click.self="handleBackdropClick"
      >
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-lg w-full max-h-[80vh] flex flex-col">
          <!-- Header -->
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white">批量章节分析</h2>
            <button
              v-if="!progress.isRunning"
              @click="handleCloseClick"
              class="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
            <!-- Show a disabled/locked indicator when running -->
            <div
              v-else
              class="p-1 text-gray-300 dark:text-gray-600"
              title="分析进行中，无法关闭"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
              </svg>
            </div>
          </div>

          <!-- Content -->
          <div class="flex-1 overflow-auto p-6 space-y-4">
            <!-- Status -->
            <div class="text-center">
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ statusText }}</p>
              <div v-if="progress.isRunning" class="mt-2 space-y-1">
                <p class="text-sm font-medium text-gray-800 dark:text-gray-200">
                  {{ progress.currentChapterTitle }}
                </p>
                <p class="text-xs text-primary-600 dark:text-primary-400">
                  当前任务: {{ progress.currentAgentType }}
                </p>
              </div>
            </div>

            <!-- Progress bar -->
            <div class="space-y-2">
              <div class="flex justify-between text-xs text-gray-500 dark:text-gray-400">
                <span>进度</span>
                <span>{{ progressPercent }}%</span>
              </div>
              <div class="h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                <div
                  class="h-full bg-primary-500 transition-all duration-300 ease-out"
                  :style="{ width: `${progressPercent}%` }"
                ></div>
              </div>
            </div>

            <!-- Selected chapters list (before starting) -->
            <div v-if="!progress.isRunning && progress.currentIndex === 0" class="space-y-2">
              <p class="text-sm font-medium text-gray-700 dark:text-gray-300">
                已选择 {{ selectedChapters.length }} 个章节:
              </p>
              <div class="max-h-40 overflow-auto bg-gray-50 dark:bg-gray-900 rounded-lg p-3">
                <ul class="space-y-1">
                  <li
                    v-for="chapter in selectedChapters"
                    :key="chapter.id"
                    class="text-xs text-gray-600 dark:text-gray-400 truncate"
                  >
                    {{ chapter.title }}
                  </li>
                </ul>
              </div>
            </div>

            <!-- Completed/Failed summary (after running) -->
            <div v-if="progress.completedChapters.length > 0 || progress.failedChapters.length > 0" class="space-y-3">
              <div v-if="progress.completedChapters.length > 0" class="text-sm">
                <span class="text-green-600 dark:text-green-400">
                  ✓ 已完成: {{ progress.completedChapters.length }} 章
                </span>
              </div>
              <div v-if="progress.failedChapters.length > 0" class="space-y-1">
                <span class="text-sm text-red-600 dark:text-red-400">
                  ✗ 失败: {{ progress.failedChapters.length }} 章
                </span>
                <ul class="text-xs text-gray-500 dark:text-gray-400 pl-4">
                  <li v-for="failed in progress.failedChapters" :key="failed.id">
                    {{ failed.title }}: {{ failed.error }}
                  </li>
                </ul>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
            <button
              v-if="!progress.isRunning && progress.currentIndex === 0"
              @click="emit('close')"
              class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            >
              取消
            </button>
            <button
              v-if="!progress.isRunning && progress.currentIndex === 0"
              @click="emit('start')"
              class="px-4 py-2 text-sm text-white bg-primary-500 hover:bg-primary-600 rounded-lg transition-colors"
            >
              开始分析
            </button>
            <button
              v-if="progress.isRunning"
              @click="emit('cancel')"
              class="px-4 py-2 text-sm text-white bg-red-500 hover:bg-red-600 rounded-lg transition-colors"
            >
              取消分析
            </button>
            <button
              v-if="!progress.isRunning && progress.currentIndex > 0"
              @click="emit('close')"
              class="px-4 py-2 text-sm text-white bg-primary-500 hover:bg-primary-600 rounded-lg transition-colors"
            >
              完成
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}
.modal-enter-active .bg-white,
.modal-leave-active .bg-white,
.modal-enter-active .dark\:bg-gray-800,
.modal-leave-active .dark\:bg-gray-800 {
  transition: transform 0.2s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
.modal-enter-from .bg-white,
.modal-leave-to .bg-white,
.modal-enter-from .dark\:bg-gray-800,
.modal-leave-to .dark\:bg-gray-800 {
  transform: scale(0.95);
}
</style>
