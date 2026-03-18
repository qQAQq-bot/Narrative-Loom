import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { useTauri } from '@/composables/useTauri';
import { useBookStore } from './book';

export interface BookInfo {
  id: string;
  title: string;
  author: string | null;
  cover_path: string | null;
  total_chapters: number;
  analyzed_chapters: number;
  status: 'importing' | 'ready' | 'analyzing' | 'completed' | 'error';
  created_at: string;
  updated_at: string;
}

export interface ImportPreview {
  title: string;
  author: string | null;
  chapter_count: number;
  total_chars: number;
  encoding: string;
  chapters: { index: number; title: string | null; char_count: number; preview: string }[];
}

export interface ImportResult {
  book: BookInfo;
  chapter_count: number;
  total_chars: number;
  encoding: string;
}

export interface LibraryStats {
  total_books: number;
  total_chapters: number;
  total_words: number;
}

// 导入任务状态
export interface ImportingBook {
  id: string; // 临时ID
  path: string;
  title: string;
  author: string | null;
  chapter_count: number;
  total_chars: number;
  progress: number; // 0-100
  status: 'parsing' | 'importing' | 'done' | 'error';
  error?: string;
  preview?: ImportPreview;
}

export const useLibraryStore = defineStore('library', () => {
  const { invoke } = useTauri();

  const books = ref<BookInfo[]>([]);
  const stats = ref<LibraryStats>({ total_books: 0, total_chapters: 0, total_words: 0 });
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  // 导入中的书籍
  const importingBooks = ref<ImportingBook[]>([]);

  // 合并显示的书籍列表（导入中的 + 已导入的）
  const allBooks = computed(() => {
    return [...importingBooks.value, ...books.value];
  });

  async function fetchBooks() {
    isLoading.value = true;
    error.value = null;
    try {
      books.value = await invoke<BookInfo[]>('list_books');
      await fetchStats();
    } catch (e) {
      error.value = 'Failed to load library';
      console.error(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function fetchStats() {
    try {
      stats.value = await invoke<LibraryStats>('get_library_stats');
    } catch (e) {
      console.error('Failed to load stats', e);
    }
  }

  async function previewImport(path: string, options?: { title?: string, author?: string, encoding?: string }) {
    return await invoke<ImportPreview>('preview_book_import', { path, ...options });
  }

  // 开始解析文件，在书架上显示
  function startParsing(path: string): string {
    const tempId = `importing-${Date.now()}`;
    const fileName = path.split(/[\\/]/).pop() || '未知文件';

    importingBooks.value.unshift({
      id: tempId,
      path,
      title: fileName,
      author: null,
      chapter_count: 0,
      total_chars: 0,
      progress: 0,
      status: 'parsing',
    });

    return tempId;
  }

  // 更新解析进度
  function updateParsingProgress(tempId: string, progress: number) {
    const book = importingBooks.value.find(b => b.id === tempId);
    if (book) {
      book.progress = progress;
    }
  }

  // 解析完成，更新预览信息
  function parsingComplete(tempId: string, preview: ImportPreview) {
    const book = importingBooks.value.find(b => b.id === tempId);
    if (book) {
      book.title = preview.title || book.title;
      book.author = preview.author;
      book.chapter_count = preview.chapter_count;
      book.total_chars = preview.total_chars;
      book.progress = 100;
      book.status = 'done';
      book.preview = preview;
    }
  }

  // 解析失败
  function parsingFailed(tempId: string, errorMsg: string) {
    const book = importingBooks.value.find(b => b.id === tempId);
    if (book) {
      book.status = 'error';
      book.error = errorMsg;
    }
  }

  // 取消导入
  function cancelImport(tempId: string) {
    importingBooks.value = importingBooks.value.filter(b => b.id !== tempId);
  }

  // 确认导入
  async function confirmImport(tempId: string, options?: { title?: string, author?: string }) {
    const importingBook = importingBooks.value.find(b => b.id === tempId);
    if (!importingBook) return;

    importingBook.status = 'importing';
    importingBook.progress = 50;

    // Animate progress from 50 to 95 while importing
    let progress = 50;
    const progressInterval = setInterval(() => {
      if (progress < 95) {
        const increment = Math.max(1, Math.floor((95 - progress) / 8));
        progress = Math.min(95, progress + increment);
        importingBook.progress = progress;
      }
    }, 300);

    try {
      const result = await invoke<ImportResult>('import_book', {
        path: importingBook.path,
        title: options?.title || importingBook.title,
        author: options?.author || importingBook.author,
      });

      // Stop animation
      clearInterval(progressInterval);

      // 导入成功，移除导入中的书籍，刷新列表
      importingBooks.value = importingBooks.value.filter(b => b.id !== tempId);
      await fetchBooks();
      return result;
    } catch (e) {
      clearInterval(progressInterval);
      importingBook.status = 'error';
      importingBook.error = String(e);
      throw e;
    }
  }

  // 获取导入中的书籍
  function getImportingBook(tempId: string) {
    return importingBooks.value.find(b => b.id === tempId);
  }

  async function importBook(path: string, options?: { title?: string, author?: string, encoding?: string }) {
    try {
      const result = await invoke<ImportResult>('import_book', { path, ...options });
      await fetchBooks(); // Refresh list
      return result;
    } catch (e) {
      console.error(e);
      throw e;
    }
  }

  async function deleteBook(bookId: string) {
    try {
      const success = await invoke<boolean>('delete_book', { bookId });
      if (success) {
        books.value = books.value.filter(b => b.id !== bookId);
        // Clean up reading record from localStorage
        const bookStore = useBookStore();
        bookStore.removeLastReadChapter(bookId);
        await fetchStats();
      }
      return success;
    } catch (e) {
      console.error(e);
      throw e;
    }
  }

  return {
    books,
    stats,
    isLoading,
    error,
    importingBooks,
    allBooks,
    fetchBooks,
    fetchStats,
    previewImport,
    importBook,
    deleteBook,
    // 新的导入流程
    startParsing,
    updateParsingProgress,
    parsingComplete,
    parsingFailed,
    cancelImport,
    confirmImport,
    getImportingBook,
  };
});
