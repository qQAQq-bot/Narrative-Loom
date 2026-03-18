<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useBookStore } from '@/stores/book';

interface VectorSearchResult {
  chunk_id: string;
  content: string;
  chapter_id: string;
  chunk_type: string;
  score: number;
  char_start: number;
  char_end: number;
  entities_mentioned: string[];
}

const props = defineProps<{
  bookId: string;
  entityId: string;
  entityName: string;
  entityType: 'character' | 'setting' | 'event';
}>();

const emit = defineEmits<{
  close: [];
  goToChapter: [chapterId: string];
}>();

const bookStore = useBookStore();
const loading = ref(false);
const error = ref<string | null>(null);
const history = ref<VectorSearchResult[]>([]);

// Group history by chapter
const historyByChapter = computed(() => {
  const grouped: Record<string, VectorSearchResult[]> = {};
  for (const item of history.value) {
    if (!grouped[item.chapter_id]) {
      grouped[item.chapter_id] = [];
    }
    grouped[item.chapter_id].push(item);
  }
  return grouped;
});

const chapterCount = computed(() => Object.keys(historyByChapter.value).length);

// Get chapter info
const getChapterInfo = (chapterId: string) => {
  const chapter = bookStore.chapters.find(c => c.id === chapterId);
  return {
    title: chapter?.title || `章节 ${chapterId.slice(0, 8)}`,
    index: chapter?.index || 0
  };
};

// Truncate content for display
const truncateContent = (content: string, maxLength: number = 150) => {
  if (content.length <= maxLength) return content;
  return content.slice(0, maxLength) + '...';
};

// Highlight entity name in content
const highlightEntity = (content: string) => {
  const escapedName = props.entityName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const regex = new RegExp(`(${escapedName})`, 'gi');
  return content.replace(regex, '<mark class="bg-yellow-200 dark:bg-yellow-800/50 px-0.5 rounded">$1</mark>');
};

// Fetch entity history
const fetchHistory = async () => {
  if (!props.bookId || !props.entityId) return;

  loading.value = true;
  error.value = null;

  try {
    const results = await invoke<VectorSearchResult[]>('get_entity_history', {
      bookId: props.bookId,
      entityId: props.entityId,
      maxPassages: 30
    });
    history.value = results || [];
  } catch (e) {
    console.error('Failed to fetch entity history:', e);
    error.value = typeof e === 'string' ? e : '加载实体历史失败';
    history.value = [];
  } finally {
    loading.value = false;
  }
};

// Watch for entity changes
watch(() => props.entityId, fetchHistory, { immediate: true });

// Navigate to chapter
const goToChapter = (chapterId: string) => {
  emit('goToChapter', chapterId);
};
</script>

<template>
  <div class="entity-history-panel bg-white dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-gray-50/50 dark:bg-gray-900/50">
      <div class="flex items-center gap-3">
        <div
          :class="[
            'w-2 h-8 rounded-full',
            entityType === 'character' ? 'bg-accent-character' :
            entityType === 'setting' ? 'bg-accent-setting' :
            'bg-accent-event'
          ]"
        />
        <div>
          <h3 class="font-bold text-gray-900 dark:text-white">{{ entityName }}</h3>
          <p class="text-xs text-gray-500 dark:text-gray-400">
            出现在 {{ chapterCount }} 个章节
          </p>
        </div>
      </div>
      <button
        @click="emit('close')"
        class="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
      >
        <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-4">
      <!-- Loading -->
      <div v-if="loading" class="flex flex-col items-center justify-center py-12 text-gray-400">
        <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-primary-400 mb-2"></div>
        <span class="text-sm">加载历史记录...</span>
      </div>

      <!-- Error -->
      <div v-else-if="error" class="text-center py-8">
        <div class="text-red-500 dark:text-red-400 text-sm">{{ error }}</div>
        <button
          @click="fetchHistory"
          class="mt-2 text-xs text-primary-500 hover:text-primary-600"
        >
          重试
        </button>
      </div>

      <!-- Empty State -->
      <div v-else-if="history.length === 0" class="flex flex-col items-center justify-center py-12 text-gray-400">
        <svg class="w-10 h-10 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
        <p class="text-sm">暂无历史记录</p>
        <p class="text-xs mt-1 text-gray-500 dark:text-gray-500">需要先生成章节的向量索引</p>
      </div>

      <!-- History List -->
      <div v-else class="space-y-4">
        <div
          v-for="(passages, chapterId) in historyByChapter"
          :key="chapterId"
          class="bg-gray-50 dark:bg-gray-900/50 rounded-lg overflow-hidden"
        >
          <!-- Chapter Header -->
          <div
            class="flex items-center justify-between px-3 py-2 bg-gray-100/50 dark:bg-gray-800/50 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
            @click="goToChapter(chapterId)"
          >
            <div class="flex items-center gap-2">
              <span class="text-xs font-mono text-gray-400 dark:text-gray-500">
                #{{ getChapterInfo(chapterId).index }}
              </span>
              <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
                {{ getChapterInfo(chapterId).title }}
              </span>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs text-gray-400 dark:text-gray-500">
                {{ passages.length }} 处提及
              </span>
              <svg class="w-4 h-4 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
              </svg>
            </div>
          </div>

          <!-- Passages -->
          <div class="divide-y divide-gray-100 dark:divide-gray-800">
            <div
              v-for="passage in passages.slice(0, 3)"
              :key="passage.chunk_id"
              class="px-3 py-2"
            >
              <p
                class="text-xs text-gray-600 dark:text-gray-400 leading-relaxed"
                v-html="highlightEntity(truncateContent(passage.content))"
              />
              <div class="flex items-center gap-2 mt-1">
                <span class="text-[10px] text-gray-400 dark:text-gray-500">
                  相关度: {{ (passage.score * 100).toFixed(0) }}%
                </span>
              </div>
            </div>
            <div
              v-if="passages.length > 3"
              class="px-3 py-2 text-xs text-gray-400 dark:text-gray-500 text-center"
            >
              还有 {{ passages.length - 3 }} 处提及...
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="px-4 py-3 border-t border-gray-200 dark:border-gray-700 bg-gray-50/50 dark:bg-gray-900/50">
      <p class="text-xs text-gray-400 dark:text-gray-500 text-center">
        点击章节标题可跳转阅读
      </p>
    </div>
  </div>
</template>

<style scoped>
.entity-history-panel {
  width: 360px;
  max-width: 100%;
}
</style>
