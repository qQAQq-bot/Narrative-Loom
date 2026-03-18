<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRoute } from 'vue-router';
import { useBookStore } from '@/stores/book';
import { useAnalysisStore } from '@/stores/analysis';
import ChapterNav from '@/components/reader/ChapterNav.vue';
import BatchAnalysisModal from '@/components/analysis/BatchAnalysisModal.vue';

const route = useRoute();
const bookStore = useBookStore();
const analysisStore = useAnalysisStore();

const showBatchModal = ref(false);

// Reload enabled agent types when component mounts (in case settings changed)
onMounted(() => {
  analysisStore.loadEnabledAgentTypes();
});

const bookId = computed(() => route.params.id as string);

const navItems = computed(() => [
  {
    path: `/book/${bookId.value}`,
    name: '阅读',
    icon: 'M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253',
    exact: true
  },
  {
    path: `/book/${bookId.value}/inbox`,
    name: '收件箱',
    icon: 'M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4'
  },
  {
    path: `/book/${bookId.value}/bible`,
    name: '圣经',
    icon: 'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z'
  },
  {
    path: `/book/${bookId.value}/techniques`,
    name: '技法',
    icon: 'M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z'
  },
  {
    path: `/book/${bookId.value}/search`,
    name: '搜索',
    icon: 'M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z'
  },
]);

function isActive(item: { path: string; exact?: boolean }): boolean {
  if (item.exact) {
    return route.path === item.path;
  }
  return route.path.startsWith(item.path);
}

function handleChapterSelect(chapterId: string) {
  bookStore.loadChapter(chapterId);
}

function handleToggleBatchSelect(chapterId: string) {
  analysisStore.toggleChapterSelection(chapterId);
}

function handleSelectVolume(volumeTitle: string) {
  analysisStore.selectVolumeChapters(volumeTitle, bookStore.chapters);
}

// Get selected chapters info for the modal
const selectedChaptersInfo = computed(() => {
  return bookStore.chapters
    .filter(c => analysisStore.selectedChapterIds.has(c.id))
    .map(c => ({ id: c.id, title: c.title || `第${c.index}章` }));
});

function openBatchModal() {
  // If batch analysis is running, just show the modal without resetting
  if (analysisStore.isBatchAnalyzing || analysisStore.batchProgress.isRunning) {
    showBatchModal.value = true;
    return;
  }

  // Otherwise, check if chapters are selected and reset progress
  if (analysisStore.selectedChapterIds.size > 0) {
    analysisStore.resetBatchProgress();
    showBatchModal.value = true;
  }
}

function closeBatchModal() {
  // Only allow closing if not running
  if (!analysisStore.batchProgress.isRunning) {
    showBatchModal.value = false;
  }
}

async function startBatchAnalysis() {
  // Uses store's enabledAgentTypes from settings
  await analysisStore.startBatchAnalysis(selectedChaptersInfo.value);
}

function cancelBatchAnalysis() {
  analysisStore.cancelBatchAnalysis();
}
</script>

<template>
  <aside class="w-56 bg-fabric-cream dark:bg-fabric-cream border-r border-fabric-sand/40 flex flex-col h-full shrink-0">
    <!-- Compact Header -->
    <div class="px-3 py-2.5 border-b border-fabric-sand/40 shrink-0">
      <div class="flex items-center justify-between">
        <router-link
          to="/"
          class="text-xs text-fabric-thread/70 hover:text-fabric-sepia flex items-center gap-0.5 transition-colors"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
          <span>书库</span>
        </router-link>

        <!-- Progress indicator -->
        <span v-if="bookStore.currentBook" class="text-xs text-fabric-thread/50">
          {{ bookStore.currentBook.analyzed_chapters }}/{{ bookStore.currentBook.total_chapters }}
        </span>
      </div>

      <h2 class="text-sm font-semibold mt-1.5 truncate text-fabric-sepia leading-tight" :title="bookStore.currentBook?.title">
        {{ bookStore.currentBook?.title || '加载中...' }}
      </h2>

      <p
        v-if="bookStore.currentBook?.author"
        class="text-xs text-fabric-thread/60 truncate mt-0.5"
      >
        {{ bookStore.currentBook.author }}
      </p>
    </div>

    <!-- Compact Horizontal Navigation Tabs -->
    <nav class="px-2 py-2 shrink-0 border-b border-fabric-sand/30">
      <div class="flex flex-wrap gap-1">
        <router-link
          v-for="item in navItems"
          :key="item.path"
          :to="item.path"
          :class="[
            'flex items-center gap-1 px-2 py-1 rounded-md text-xs font-medium transition-all duration-150',
            isActive(item)
              ? 'bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300 shadow-sm'
              : 'text-fabric-thread/70 hover:bg-fabric-sand/40 hover:text-fabric-sepia',
          ]"
          :title="item.name"
        >
          <svg class="w-3.5 h-3.5 shrink-0" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" :d="item.icon" />
          </svg>
          <span>{{ item.name }}</span>
        </router-link>
      </div>
    </nav>

    <!-- Chapter List Header -->
    <div class="px-3 py-2 flex items-center justify-between shrink-0 bg-fabric-sand/20">
      <span class="text-xs font-medium text-fabric-thread/70">章节目录</span>
      <div class="flex items-center gap-1">
        <span class="text-xs text-fabric-thread/50">{{ bookStore.totalChapters }} 章</span>
        <!-- Batch mode toggle button -->
        <button
          @click="analysisStore.toggleBatchMode()"
          :class="[
            'p-1 rounded transition-colors',
            analysisStore.batchMode
              ? 'bg-primary-100 text-primary-600 dark:bg-primary-900/30 dark:text-primary-400'
              : 'text-fabric-thread/50 hover:text-fabric-thread hover:bg-fabric-sand/40'
          ]"
          title="批量选择模式"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Batch mode action bar -->
    <div v-if="analysisStore.batchMode" class="px-3 py-2 bg-primary-50 dark:bg-primary-900/20 border-b border-primary-200 dark:border-primary-800 shrink-0">
      <div class="flex items-center justify-between gap-2">
        <span class="text-xs text-primary-700 dark:text-primary-300">
          已选 {{ analysisStore.selectedChapterIds.size }} 章
        </span>
        <div class="flex items-center gap-1">
          <button
            v-if="analysisStore.selectedChapterIds.size > 0"
            @click="analysisStore.clearChapterSelection()"
            class="px-2 py-1 text-xs text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
          >
            清除
          </button>
          <button
            @click="openBatchModal()"
            :disabled="analysisStore.selectedChapterIds.size === 0 && !analysisStore.isBatchAnalyzing"
            :class="[
              'px-2 py-1 text-xs rounded transition-colors relative',
              analysisStore.isBatchAnalyzing
                ? 'bg-amber-500 text-white hover:bg-amber-600'
                : analysisStore.selectedChapterIds.size > 0
                  ? 'bg-primary-500 text-white hover:bg-primary-600'
                  : 'bg-gray-200 text-gray-400 cursor-not-allowed dark:bg-gray-700 dark:text-gray-500'
            ]"
          >
            {{ analysisStore.isBatchAnalyzing ? '查看进度' : '批量分析' }}
            <!-- Pulsing indicator when running -->
            <span
              v-if="analysisStore.isBatchAnalyzing"
              class="absolute -top-1 -right-1 w-2.5 h-2.5 bg-red-500 rounded-full animate-pulse"
            ></span>
          </button>
        </div>
      </div>
    </div>

    <!-- Chapter List - Takes remaining space -->
    <div class="flex-1 min-h-0 overflow-y-auto">
      <div class="px-2 py-1">
        <div v-if="bookStore.isLoadingBook" class="text-center text-fabric-thread/50 py-6">
          <span class="animate-pulse text-xs">加载中...</span>
        </div>

        <ChapterNav
          v-else
          :chapters="bookStore.chapters"
          :current-chapter-id="bookStore.currentChapterId"
          :batch-mode="analysisStore.batchMode"
          :selected-chapter-ids="analysisStore.selectedChapterIds"
          @select="handleChapterSelect"
          @toggle-batch-select="handleToggleBatchSelect"
          @select-volume="handleSelectVolume"
        />
      </div>
    </div>

    <!-- Batch Analysis Modal -->
    <BatchAnalysisModal
      :show="showBatchModal"
      :progress="analysisStore.batchProgress"
      :selected-chapters="selectedChaptersInfo"
      @close="closeBatchModal"
      @cancel="cancelBatchAnalysis"
      @start="startBatchAnalysis"
    />
  </aside>
</template>
