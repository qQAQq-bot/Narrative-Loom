<script setup lang="ts">
import type { ImportingBook } from '@/stores/library';

defineProps<{
  book: ImportingBook;
}>();

const emit = defineEmits<{
  (e: 'cancel', book: ImportingBook): void;
  (e: 'confirm', book: ImportingBook): void;
}>();

const getStatusText = (status: ImportingBook['status']) => {
  switch (status) {
    case 'parsing': return '解析中...';
    case 'importing': return '导入中...';
    case 'done': return '解析完成';
    case 'error': return '解析失败';
    default: return '';
  }
};
</script>

<template>
  <div
    class="group relative flex flex-col fabric-card overflow-hidden transition-all duration-300"
    :class="{
      'ring-2 ring-red-400/50 shadow-lg shadow-red-500/10': book.status === 'error',
      'ring-2 ring-green-400/30 shadow-lg shadow-green-500/10': book.status === 'done'
    }"
  >
    <!-- Cover Area - same structure as BookCard -->
    <div class="aspect-[3/4] w-full relative overflow-hidden">
      <!-- Background with gradient -->
      <div class="absolute inset-0 bg-gradient-to-br from-fabric-linen via-fabric-warm to-fabric-sand"></div>
      <!-- Fabric texture pattern -->
      <div class="absolute inset-0 bg-canvas opacity-30"></div>
      <!-- Decorative stitch pattern -->
      <div class="absolute inset-0 opacity-15 bg-[radial-gradient(circle_at_1px_1px,#8b7355_1px,transparent_0)] bg-[size:12px_12px]"></div>

      <!-- Book icon placeholder -->
      <div class="absolute inset-0 flex items-center justify-center">
        <div class="w-20 h-20 rounded-2xl bg-fabric-warm/60 backdrop-blur-sm flex items-center justify-center shadow-inner">
          <svg class="w-10 h-10 text-fabric-thread/40" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
          </svg>
        </div>
      </div>

      <!-- Loading overlay for parsing/importing -->
      <div
        v-if="book.status === 'parsing' || book.status === 'importing'"
        class="absolute inset-0 bg-fabric-cream/70 dark:bg-fabric-canvas/70 backdrop-blur-[3px] flex flex-col items-center justify-center"
      >
        <!-- Animated spinner -->
        <div class="relative w-14 h-14 mb-3">
          <div class="absolute inset-0 border-3 border-primary-200 dark:border-primary-800 rounded-full"></div>
          <div class="absolute inset-0 border-3 border-primary-500 border-t-transparent rounded-full animate-spin"></div>
          <div class="absolute inset-2 border-2 border-primary-300 border-b-transparent rounded-full animate-spin" style="animation-direction: reverse; animation-duration: 1.5s;"></div>
        </div>
        <span class="text-xs text-fabric-thread/80 font-medium">{{ getStatusText(book.status) }}</span>
        <span class="text-xs text-primary-500 font-mono mt-1">{{ book.progress }}%</span>
      </div>

      <!-- Done overlay -->
      <div
        v-else-if="book.status === 'done'"
        class="absolute inset-0 bg-green-50/80 dark:bg-green-900/40 backdrop-blur-[3px] flex flex-col items-center justify-center"
      >
        <div class="w-14 h-14 bg-green-100 dark:bg-green-800/60 rounded-full flex items-center justify-center mb-3 shadow-lg shadow-green-500/20">
          <svg class="w-7 h-7 text-green-600 dark:text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
          </svg>
        </div>
        <span class="text-sm text-green-700 dark:text-green-300 font-medium">{{ getStatusText(book.status) }}</span>
        <span class="text-xs text-green-600/70 dark:text-green-400/70 mt-0.5">{{ book.chapter_count }} 章节</span>
      </div>

      <!-- Error overlay -->
      <div
        v-else-if="book.status === 'error'"
        class="absolute inset-0 bg-red-50/80 dark:bg-red-900/40 backdrop-blur-[3px] flex flex-col items-center justify-center"
      >
        <div class="w-14 h-14 bg-red-100 dark:bg-red-800/60 rounded-full flex items-center justify-center mb-3 shadow-lg shadow-red-500/20">
          <svg class="w-7 h-7 text-red-600 dark:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </div>
        <span class="text-sm text-red-700 dark:text-red-300 font-medium">{{ getStatusText(book.status) }}</span>
      </div>

      <!-- Cancel Button - always visible on hover or when error -->
      <button
        @click.stop="emit('cancel', book)"
        class="absolute top-2 right-2 transition-all duration-220 z-10
               w-7 h-7 bg-fabric-warm/95 hover:bg-red-50 dark:hover:bg-red-900/50
               text-fabric-thread hover:text-red-600 dark:hover:text-red-400
               rounded-full shadow-fabric flex items-center justify-center
               opacity-0 group-hover:opacity-100"
        :class="{ 'opacity-100': book.status === 'error' }"
        title="取消导入"
      >
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2.5" stroke="currentColor" class="w-3.5 h-3.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>

      <!-- Confirm Button for done state -->
      <div v-if="book.status === 'done'" class="absolute bottom-2 right-2 z-10">
        <button
          @click.stop="emit('confirm', book)"
          class="px-3 py-1.5 bg-green-500 hover:bg-green-600 text-white text-xs font-medium rounded-full shadow-lg shadow-green-500/30 transition-all duration-150 flex items-center gap-1.5 hover:scale-105"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
          </svg>
          确认导入
        </button>
      </div>
    </div>

    <!-- Content - same structure as BookCard -->
    <div class="p-3 flex flex-col flex-1 bg-fabric-warm">
      <h3 class="text-sm font-bold text-fabric-sepia leading-tight line-clamp-2 mb-1.5 font-serif">
        {{ book.title }}
      </h3>

      <div class="flex items-center justify-between mt-auto">
        <div class="flex items-center text-xs text-fabric-thread/70">
          <svg class="w-3 h-3 mr-1 opacity-60" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
          </svg>
          <span class="truncate max-w-[6rem]">{{ book.author || '未知作者' }}</span>
        </div>

        <div v-if="book.status === 'done'" class="text-xs font-medium text-green-600 dark:text-green-400 font-mono">
          {{ book.chapter_count }}章
        </div>
        <div v-else class="text-xs font-medium text-primary-500 font-mono">
          {{ book.progress }}%
        </div>
      </div>

      <!-- Progress Bar -->
      <div class="mt-2 h-1 w-full bg-fabric-sand/40 rounded-full overflow-hidden">
        <div
          class="h-full transition-all duration-300"
          :class="{
            'bg-primary-400': book.status === 'parsing' || book.status === 'importing',
            'bg-green-500': book.status === 'done',
            'bg-red-400': book.status === 'error',
          }"
          :style="{ width: `${book.progress}%` }"
        ></div>
      </div>

      <!-- Error message -->
      <p v-if="book.status === 'error' && book.error" class="mt-1.5 text-xs text-red-600 dark:text-red-400 line-clamp-1">
        {{ book.error }}
      </p>
    </div>
  </div>
</template>
