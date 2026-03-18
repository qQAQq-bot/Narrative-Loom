import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { useTauri } from '@/composables/useTauri';
import { useLibraryStore, type BookInfo } from './library';

export interface ChapterListItem {
  id: string;
  index: number;
  title: string | null;
  parent_title: string | null;
  char_count: number;
  analyzed: boolean;
}

export interface ChapterDetail {
  id: string;
  book_id: string;
  index: number;
  title: string | null;
  parent_title: string | null;
  char_count: number;
  analyzed: boolean;
  technique_count: number;
  knowledge_count: number;
  content: string;
}

export interface AdjacentChapters {
  prev: ChapterListItem | null;
  next: ChapterListItem | null;
}

// LRU Cache for chapter content
class ChapterCache {
  private cache = new Map<string, ChapterDetail>();
  private readonly maxSize: number;

  constructor(maxSize = 10) {
    this.maxSize = maxSize;
  }

  get(key: string): ChapterDetail | undefined {
    const value = this.cache.get(key);
    if (value) {
      // Move to end (most recently used)
      this.cache.delete(key);
      this.cache.set(key, value);
    }
    return value;
  }

  set(key: string, value: ChapterDetail): void {
    if (this.cache.has(key)) {
      this.cache.delete(key);
    } else if (this.cache.size >= this.maxSize) {
      // Remove oldest (first) entry
      const firstKey = this.cache.keys().next().value;
      if (firstKey) {
        this.cache.delete(firstKey);
      }
    }
    this.cache.set(key, value);
  }

  clear(): void {
    this.cache.clear();
  }
}

export const useBookStore = defineStore('book', () => {
  const { invoke } = useTauri();
  const libraryStore = useLibraryStore();

  // Chapter content cache (LRU, max 10 chapters)
  const chapterCache = new ChapterCache(10);

  // Current book state
  const currentBookId = ref<string | null>(null);
  const currentBook = ref<BookInfo | null>(null);
  const chapters = ref<ChapterListItem[]>([]);

  // Current chapter state
  const currentChapterId = ref<string | null>(null);
  const currentChapter = ref<ChapterDetail | null>(null);
  const adjacentChapters = ref<AdjacentChapters>({ prev: null, next: null });

  // Loading states
  const isLoadingBook = ref(false);
  const isLoadingChapter = ref(false);
  const error = ref<string | null>(null);

  // Local storage key for last read chapter
  const LAST_READ_KEY = 'narrative-loom:last-read-chapter';

  // Save last read chapter to localStorage
  function saveLastReadChapter(bookId: string, chapterId: string) {
    try {
      const data = JSON.parse(localStorage.getItem(LAST_READ_KEY) || '{}');
      data[bookId] = chapterId;
      localStorage.setItem(LAST_READ_KEY, JSON.stringify(data));
    } catch (e) {
      console.error('Failed to save last read chapter:', e);
    }
  }

  // Remove last read chapter record for a book (used when deleting a book)
  function removeLastReadChapter(bookId: string) {
    try {
      const data = JSON.parse(localStorage.getItem(LAST_READ_KEY) || '{}');
      delete data[bookId];
      localStorage.setItem(LAST_READ_KEY, JSON.stringify(data));
    } catch (e) {
      console.error('Failed to remove last read chapter:', e);
    }
  }

  // Get last read chapter from localStorage
  function getLastReadChapter(bookId: string): string | null {
    try {
      const data = JSON.parse(localStorage.getItem(LAST_READ_KEY) || '{}');
      return data[bookId] || null;
    } catch (e) {
      console.error('Failed to get last read chapter:', e);
      return null;
    }
  }

  // Computed
  const currentChapterIndex = computed(() => currentChapter.value?.index ?? 0);
  const totalChapters = computed(() => chapters.value.length);
  const hasNextChapter = computed(() => adjacentChapters.value.next !== null);
  const hasPrevChapter = computed(() => adjacentChapters.value.prev !== null);

  const chapterTitle = computed(() => {
    if (!currentChapter.value) return '';
    return currentChapter.value.title || `第${currentChapter.value.index}章`;
  });

  // Actions
  async function loadBook(bookId: string) {
    if (currentBookId.value === bookId && chapters.value.length > 0) {
      return; // Already loaded
    }

    isLoadingBook.value = true;
    error.value = null;
    currentBookId.value = bookId;

    try {
      // Get book info from library store
      if (libraryStore.books.length === 0) {
        await libraryStore.fetchBooks();
      }
      currentBook.value = libraryStore.books.find(b => b.id === bookId) || null;

      // Fetch chapters
      chapters.value = await invoke<ChapterListItem[]>('get_chapters', { bookId });
    } catch (e) {
      error.value = '加载书籍失败';
      console.error(e);
    } finally {
      isLoadingBook.value = false;
    }
  }

  // Force refresh book info (for updating analyzed_chapters count)
  async function refreshBook() {
    if (!currentBookId.value) return;

    try {
      // Refresh library to get updated book info
      await libraryStore.fetchBooks();
      currentBook.value = libraryStore.books.find(b => b.id === currentBookId.value) || null;
    } catch (e) {
      console.error('Failed to refresh book:', e);
    }
  }

  async function loadChapter(chapterId: string) {
    if (!currentBookId.value) {
      error.value = '请先选择书籍';
      return;
    }

    if (currentChapterId.value === chapterId && currentChapter.value) {
      return; // Already loaded
    }

    currentChapterId.value = chapterId;

    // Save as last read chapter
    saveLastReadChapter(currentBookId.value, chapterId);

    // Check cache first
    const cached = chapterCache.get(chapterId);
    if (cached) {
      currentChapter.value = cached;
      // Fetch adjacent chapters in background
      fetchAdjacentChapters(cached.index);
      // Preload adjacent chapters
      preloadAdjacentChapters(cached.index);
      return;
    }

    isLoadingChapter.value = true;
    error.value = null;

    try {
      // Fetch chapter and adjacent info in parallel
      const [chapter, adjacent] = await Promise.all([
        invoke<ChapterDetail>('get_chapter', {
          bookId: currentBookId.value,
          chapterId,
        }),
        invoke<AdjacentChapters>('get_adjacent_chapters', {
          bookId: currentBookId.value,
          currentIndex: chapters.value.find(c => c.id === chapterId)?.index ?? 1,
        }),
      ]);

      currentChapter.value = chapter;
      adjacentChapters.value = adjacent;

      // Cache the chapter
      chapterCache.set(chapterId, chapter);

      // Preload adjacent chapters in background
      preloadAdjacentChapters(chapter.index);
    } catch (e) {
      error.value = '加载章节失败';
      console.error(e);
    } finally {
      isLoadingChapter.value = false;
    }
  }

  // Fetch adjacent chapters info (for navigation)
  async function fetchAdjacentChapters(currentIndex: number) {
    if (!currentBookId.value) return;
    try {
      adjacentChapters.value = await invoke<AdjacentChapters>('get_adjacent_chapters', {
        bookId: currentBookId.value,
        currentIndex,
      });
    } catch (e) {
      console.error('Failed to fetch adjacent chapters:', e);
    }
  }

  // Preload adjacent chapter content in background
  async function preloadAdjacentChapters(currentIndex: number) {
    if (!currentBookId.value) return;

    const preloadChapter = async (chapterId: string) => {
      if (chapterCache.get(chapterId)) return; // Already cached
      try {
        const chapter = await invoke<ChapterDetail>('get_chapter', {
          bookId: currentBookId.value,
          chapterId,
        });
        chapterCache.set(chapterId, chapter);
      } catch (e) {
        // Silently ignore preload errors
      }
    };

    // Find prev and next chapter IDs
    const prevChapter = chapters.value.find(c => c.index === currentIndex - 1);
    const nextChapter = chapters.value.find(c => c.index === currentIndex + 1);

    // Preload in background (don't await)
    if (nextChapter) {
      preloadChapter(nextChapter.id);
    }
    if (prevChapter) {
      preloadChapter(prevChapter.id);
    }
  }

  async function loadChapterByIndex(index: number) {
    if (!currentBookId.value) {
      error.value = '请先选择书籍';
      return;
    }

    // Find chapter by index
    const targetChapter = chapters.value.find(c => c.index === index);
    if (targetChapter) {
      // Check cache first
      const cached = chapterCache.get(targetChapter.id);
      if (cached) {
        currentChapter.value = cached;
        currentChapterId.value = cached.id;
        fetchAdjacentChapters(index);
        preloadAdjacentChapters(index);
        return;
      }
    }

    isLoadingChapter.value = true;
    error.value = null;

    try {
      currentChapter.value = await invoke<ChapterDetail>('get_chapter_by_index', {
        bookId: currentBookId.value,
        index,
      });
      currentChapterId.value = currentChapter.value.id;

      // Cache the chapter
      chapterCache.set(currentChapter.value.id, currentChapter.value);

      // Fetch adjacent chapters for navigation
      adjacentChapters.value = await invoke<AdjacentChapters>('get_adjacent_chapters', {
        bookId: currentBookId.value,
        currentIndex: index,
      });

      // Preload adjacent chapters
      preloadAdjacentChapters(index);
    } catch (e) {
      error.value = '加载章节失败';
      console.error(e);
    } finally {
      isLoadingChapter.value = false;
    }
  }

  async function goToNextChapter() {
    if (adjacentChapters.value.next) {
      await loadChapter(adjacentChapters.value.next.id);
    }
  }

  async function goToPrevChapter() {
    if (adjacentChapters.value.prev) {
      await loadChapter(adjacentChapters.value.prev.id);
    }
  }

  // Refresh chapters list (for updating analyzed status)
  async function loadChapters() {
    if (!currentBookId.value) return;
    try {
      chapters.value = await invoke<ChapterListItem[]>('get_chapters', { bookId: currentBookId.value });
    } catch (e) {
      console.error('Failed to refresh chapters:', e);
    }
  }

  function reset() {
    currentBookId.value = null;
    currentBook.value = null;
    chapters.value = [];
    currentChapterId.value = null;
    currentChapter.value = null;
    adjacentChapters.value = { prev: null, next: null };
    error.value = null;
    chapterCache.clear();
  }

  return {
    // State
    currentBookId,
    currentBook,
    chapters,
    currentChapterId,
    currentChapter,
    adjacentChapters,
    isLoadingBook,
    isLoadingChapter,
    error,

    // Computed
    currentChapterIndex,
    totalChapters,
    hasNextChapter,
    hasPrevChapter,
    chapterTitle,

    // Actions
    loadBook,
    loadChapter,
    loadChapterByIndex,
    loadChapters,
    refreshBook,
    goToNextChapter,
    goToPrevChapter,
    reset,
    getLastReadChapter,
    removeLastReadChapter,
  };
});
