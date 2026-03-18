<script setup lang="ts">
import type { BookInfo, ImportingBook } from '@/stores/library';
import BookCard from './BookCard.vue';
import ImportingBookCard from './ImportingBookCard.vue';
import Button from '@/components/ui/Button.vue';

defineProps<{
  books: BookInfo[];
  importingBooks?: ImportingBook[];
  loading?: boolean;
}>();

const emit = defineEmits<{
  (e: 'open', book: BookInfo): void;
  (e: 'delete', book: BookInfo): void;
  (e: 'import'): void;
  (e: 'cancelImport', book: ImportingBook): void;
  (e: 'confirmImport', book: ImportingBook): void;
}>();
</script>

<template>
  <div class="w-full">
    <!-- Loading State with fabric styling -->
    <div v-if="loading" class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-6">
      <div v-for="i in 10" :key="i" class="animate-pulse">
        <div class="aspect-[3/4] bg-fabric-sand/30 rounded-xl mb-3"></div>
        <div class="h-4 bg-fabric-sand/30 rounded w-3/4 mb-2"></div>
        <div class="h-3 bg-fabric-sand/30 rounded w-1/2"></div>
      </div>
    </div>

    <!-- Empty State with fabric styling -->
    <div v-else-if="books.length === 0 && (!importingBooks || importingBooks.length === 0)" class="fabric-card stitch-border flex flex-col items-center justify-center py-20 px-4">
      <!-- Illustration with fabric feel -->
      <div class="relative mb-8">
        <div class="absolute inset-0 bg-primary-500/10 blur-3xl rounded-full"></div>
        <div class="relative w-32 h-32 bg-fabric-linen rounded-3xl flex items-center justify-center border border-dashed border-fabric-thread/30">
          <svg class="w-16 h-16 text-fabric-thread/60" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
          </svg>
        </div>
      </div>

      <h3 class="text-xl font-bold text-fabric-sepia mb-2 font-serif">书库空空如也</h3>
      <p class="text-fabric-thread/70 mb-8 text-center max-w-sm">
        导入您的第一本小说，开始 AI 辅助创作之旅。支持 TXT、EPUB 格式。
      </p>

      <Button variant="primary" size="lg" @click="emit('import')">
        <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        导入书籍
      </Button>

      <!-- Supported formats hint with fabric tags -->
      <div class="mt-6 flex items-center gap-3 text-xs text-fabric-thread/60">
        <span class="fabric-tag">.txt</span>
        <span class="fabric-tag">.epub</span>
      </div>
    </div>

    <!-- Grid Layout -->
    <div v-else class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-6">
      <!-- Importing Books (shown first) -->
      <ImportingBookCard
        v-for="book in importingBooks"
        :key="book.id"
        :book="book"
        @cancel="emit('cancelImport', book)"
        @confirm="emit('confirmImport', book)"
      />

      <!-- Regular Books -->
      <BookCard
        v-for="book in books"
        :key="book.id"
        :book="book"
        @click="emit('open', book)"
        @delete="emit('delete', book)"
      />

      <!-- Add New Card (Ghost) with fabric styling - matches BookCard structure -->
      <div
        class="group relative flex flex-col fabric-card overflow-hidden cursor-pointer transform transition-all duration-300 hover:scale-[1.02] hover:shadow-xl"
        @click="emit('import')"
      >
        <!-- Cover Area - same aspect ratio as BookCard -->
        <div class="aspect-[3/4] w-full relative overflow-hidden">
          <!-- Background pattern -->
          <div class="absolute inset-0 bg-gradient-to-br from-fabric-linen via-fabric-warm to-fabric-sand"></div>
          <div class="absolute inset-0 bg-canvas opacity-30"></div>
          <!-- Dashed border overlay -->
          <div class="absolute inset-2 border-2 border-dashed border-fabric-sand/60 group-hover:border-primary-400 rounded-lg transition-colors"></div>

          <!-- Content -->
          <div class="absolute inset-0 flex flex-col items-center justify-center text-fabric-thread/60 group-hover:text-primary-600 transition-colors">
            <div class="w-12 h-12 rounded-full bg-fabric-linen group-hover:bg-fabric-warm flex items-center justify-center mb-3 transition-colors shadow-fabric-inner">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-6 h-6">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
              </svg>
            </div>
            <span class="font-medium text-sm">导入新书</span>
            <!-- Format hints -->
            <div class="mt-2 flex items-center gap-1.5 text-[10px] opacity-60">
              <span class="px-1.5 py-0.5 bg-fabric-sand/30 rounded">.txt</span>
              <span class="px-1.5 py-0.5 bg-fabric-sand/30 rounded">.epub</span>
            </div>
          </div>
        </div>

        <!-- Content Area - matches BookCard structure for consistent height -->
        <div class="p-3 flex flex-col flex-1 bg-fabric-warm">
          <h3 class="text-sm font-bold text-fabric-thread/50 leading-tight line-clamp-2 mb-1.5 font-serif">
            点击导入书籍
          </h3>

          <div class="flex items-center justify-between mt-auto">
            <div class="flex items-center text-xs text-fabric-thread/40">
              <svg class="w-3 h-3 mr-1 opacity-60" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <span>TXT / EPUB</span>
            </div>
          </div>

          <!-- Placeholder for progress bar -->
          <div class="mt-2 h-1 w-full bg-fabric-sand/20 rounded-full overflow-hidden"></div>
        </div>
      </div>
    </div>
  </div>
</template>
