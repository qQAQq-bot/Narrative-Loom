<script setup lang="ts">
import { onMounted, watch, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useBookStore } from '@/stores/book';
import { useAnalysisStore } from '@/stores/analysis';
import Sidebar from '@/components/common/Sidebar.vue';

const route = useRoute();
const router = useRouter();
const bookStore = useBookStore();
const analysisStore = useAnalysisStore();

const bookId = computed(() => route.params.id as string);

// 加载书籍和章节的逻辑
async function loadBookAndChapter(id: string) {
  await bookStore.loadBook(id);
  // 书籍加载完成后，自动加载章节
  if (bookStore.chapters.length > 0 && !bookStore.currentChapterId) {
    // 尝试加载上次阅读的章节，如果没有则加载第一章
    const lastReadChapterId = bookStore.getLastReadChapter(id);
    const chapterToLoad = lastReadChapterId && bookStore.chapters.find(c => c.id === lastReadChapterId)
      ? lastReadChapterId
      : bookStore.chapters[0].id;
    await bookStore.loadChapter(chapterToLoad);
  }

  // 如果当前不在阅读标签（默认路由），跳转到阅读标签
  if (route.name !== 'book-chapter') {
    router.replace({ name: 'book-chapter', params: { id } });
  }
}

onMounted(async () => {
  // Reset store when entering a new book to ensure clean state
  bookStore.reset();
  await loadBookAndChapter(bookId.value);
});

// 监听 bookId 变化（当用户从书架切换到不同书籍时）
watch(bookId, async (newId, oldId) => {
  if (newId && newId !== oldId) {
    // 重置当前章节，确保加载新书籍的上次阅读章节
    bookStore.reset();
    await loadBookAndChapter(newId);
  }
});

// Watch for chapter changes to load cards
watch(
  () => bookStore.currentChapterId,
  async (newChapterId) => {
    if (newChapterId) {
      await analysisStore.loadCardsForChapter(newChapterId);
    }
  }
);
</script>

<template>
  <div class="flex h-full bg-gray-50 dark:bg-gray-900">
    <Sidebar />

    <main class="flex-1 flex flex-col min-w-0 overflow-hidden">
      <router-view />
    </main>
  </div>
</template>
