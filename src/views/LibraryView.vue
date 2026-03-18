<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useRouter } from "vue-router";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useLibraryStore } from "@/stores/library";
import type { BookInfo, ImportingBook } from "@/stores/library";

import BookShelf from "@/components/library/BookShelf.vue";
import ImportConfirmModal from "@/components/library/ImportConfirmModal.vue";
import ConfirmModal from "@/components/ui/ConfirmModal.vue";
import Button from "@/components/ui/Button.vue";

const router = useRouter();
const store = useLibraryStore();

// Confirm modal state
const showConfirmModal = ref(false);
const confirmingBook = ref<ImportingBook | null>(null);

// Delete confirm modal state
const showDeleteModal = ref(false);
const deletingBook = ref<BookInfo | null>(null);
const deletingBookCover = ref<string | null>(null);

// Stat card definitions with fabric theme
const statsCards = computed(() => [
  {
    label: '书籍',
    value: store.stats?.total_books ?? 0,
    icon: 'book',
    accentColor: 'bg-primary-500/10',
    iconColor: 'text-primary-600'
  },
  {
    label: '章节',
    value: store.stats?.total_chapters ?? 0,
    icon: 'document',
    accentColor: 'bg-accent-technique/10',
    iconColor: 'text-accent-technique'
  },
  {
    label: '字数',
    value: formatChars(store.stats?.total_words ?? 0),
    icon: 'text',
    accentColor: 'bg-accent-character/10',
    iconColor: 'text-accent-character'
  },
]);

function formatChars(chars: number | undefined): string {
  if (chars == null) return '0';
  if (chars >= 10000) {
    return (chars / 10000).toFixed(1) + "万";
  }
  return chars.toLocaleString();
}

function handleOpenBook(book: BookInfo) {
  router.push({ name: "book", params: { id: book.id } });
}

async function handleDeleteBook(book: BookInfo) {
  deletingBook.value = book;
  deletingBookCover.value = null;
  showDeleteModal.value = true;

  // Fetch cover in background
  if (book.cover_path) {
    try {
      const cover = await invoke<string | null>('get_book_cover', { bookId: book.id });
      deletingBookCover.value = cover;
    } catch (e) {
      console.error('Failed to load cover:', e);
    }
  }
}

async function confirmDeleteBook() {
  if (!deletingBook.value) return;

  try {
    await store.deleteBook(deletingBook.value.id);
  } catch (e) {
    console.error("Delete failed:", e);
  } finally {
    showDeleteModal.value = false;
    deletingBook.value = null;
  }
}

function cancelDeleteBook() {
  showDeleteModal.value = false;
  deletingBook.value = null;
  deletingBookCover.value = null;
}

// New import flow functions
async function handleStartImport() {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        { name: "Supported Files", extensions: ["txt", "epub"] },
        { name: "Text Files", extensions: ["txt"] },
        { name: "EPUB Files", extensions: ["epub"] },
      ],
    });

    if (selected && typeof selected === "string") {
      // Add importing book to shelf immediately
      const tempId = store.startParsing(selected);

      // Start progress animation
      let progress = 0;
      const progressInterval = setInterval(() => {
        // Gradually increase progress, slowing down as it approaches 90%
        if (progress < 90) {
          const increment = Math.max(1, Math.floor((90 - progress) / 10));
          progress = Math.min(90, progress + increment);
          store.updateParsingProgress(tempId, progress);
        }
      }, 200);

      // Start parsing in background
      try {
        const preview = await store.previewImport(selected);

        // Stop animation and complete
        clearInterval(progressInterval);
        store.updateParsingProgress(tempId, 100);
        store.parsingComplete(tempId, preview);

        // Show confirm modal
        const importingBook = store.getImportingBook(tempId);
        if (importingBook) {
          confirmingBook.value = importingBook;
          showConfirmModal.value = true;
        }
      } catch (e) {
        clearInterval(progressInterval);
        store.parsingFailed(tempId, String(e));
      }
    }
  } catch (e) {
    console.error("File selection failed:", e);
  }
}

function handleCancelImport(book: ImportingBook) {
  store.cancelImport(book.id);
  // Close confirm modal if it's the same book
  if (confirmingBook.value?.id === book.id) {
    showConfirmModal.value = false;
    confirmingBook.value = null;
  }
}

function handleShowConfirmModal(book: ImportingBook) {
  confirmingBook.value = book;
  showConfirmModal.value = true;
}

async function handleConfirmImport(options: { title: string; author: string | null }) {
  if (!confirmingBook.value) return;

  const bookToImport = confirmingBook.value;

  // 立即关闭弹窗
  showConfirmModal.value = false;
  confirmingBook.value = null;

  // 在后台执行导入
  try {
    await store.confirmImport(bookToImport.id, {
      title: options.title,
      author: options.author || undefined,
    });
  } catch (e) {
    console.error("Import failed:", e);
  }
}

async function handleReselectFile() {
  const currentBookId = confirmingBook.value?.id;
  showConfirmModal.value = false;
  confirmingBook.value = null;

  // Cancel current import
  if (currentBookId) {
    store.cancelImport(currentBookId);
  }

  // Start new import
  await handleStartImport();
}

onMounted(() => {
  store.fetchBooks();
});
</script>

<template>
  <div class="h-full overflow-auto bg-fabric-cream bg-weave">
    <main class="max-w-7xl mx-auto px-6 py-6 space-y-6">
      <!-- Stats Overview with fabric styling -->
      <section aria-label="Library Statistics" class="grid grid-cols-3 gap-4">
        <div
          v-for="(stat, index) in statsCards"
          :key="index"
          class="fabric-card p-5 stitch-border group"
        >
          <div class="relative flex items-center gap-4">
            <!-- Icon with fabric accent -->
            <div class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0" :class="stat.accentColor">
              <svg v-if="stat.icon === 'book'" class="w-6 h-6" :class="stat.iconColor" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
              </svg>
              <svg v-else-if="stat.icon === 'document'" class="w-6 h-6" :class="stat.iconColor" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <svg v-else class="w-6 h-6" :class="stat.iconColor" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 6h16M4 12h16m-7 6h7" />
              </svg>
            </div>

            <!-- Stats -->
            <div>
              <div class="text-2xl font-bold text-fabric-sepia font-serif">
                {{ stat.value }}
              </div>
              <div class="text-xs text-fabric-thread/70 font-medium">
                {{ stat.label }}
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- Library Section -->
      <section class="space-y-4">
        <div class="flex items-center justify-between">
          <div class="flex items-baseline gap-3">
            <h2 class="text-xl font-bold text-fabric-sepia font-serif">我的书库</h2>
            <span class="text-sm text-fabric-thread/60">{{ store.books.length }} 本书</span>
          </div>

          <Button variant="primary" size="md" @click="handleStartImport">
            <svg class="w-4 h-4 mr-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            导入书籍
          </Button>
        </div>

        <!-- Book Shelf -->
        <BookShelf
          :books="store.books"
          :importing-books="store.importingBooks"
          :loading="store.isLoading"
          @open="handleOpenBook"
          @delete="handleDeleteBook"
          @import="handleStartImport"
          @cancel-import="handleCancelImport"
          @confirm-import="handleShowConfirmModal"
        />
      </section>
    </main>

    <!-- Import Confirm Modal -->
    <ImportConfirmModal
      :show="showConfirmModal"
      :book="confirmingBook"
      @close="showConfirmModal = false; confirmingBook = null"
      @confirm="handleConfirmImport"
      @reselect="handleReselectFile"
    />

    <!-- Delete Confirm Modal -->
    <ConfirmModal
      :visible="showDeleteModal"
      title="删除书籍"
      :message="`此操作将同时删除所有章节和分析数据，且不可撤销。`"
      confirm-text="确认删除"
      cancel-text="取消"
      type="danger"
      @confirm="confirmDeleteBook"
      @cancel="cancelDeleteBook"
    >
      <template #content>
        <!-- Book info preview -->
        <div v-if="deletingBook" class="flex items-center gap-3 p-3 mb-4 bg-fabric-sand/20 rounded-xl border border-fabric-sand/30">
          <!-- Mini cover -->
          <div class="w-12 h-16 rounded-lg overflow-hidden bg-fabric-linen flex-shrink-0 shadow-sm">
            <img
              v-if="deletingBookCover"
              :src="deletingBookCover"
              :alt="deletingBook.title"
              class="w-full h-full object-cover"
            />
            <div v-else class="w-full h-full flex items-center justify-center">
              <svg class="w-6 h-6 text-fabric-thread/40" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
              </svg>
            </div>
          </div>
          <!-- Book info -->
          <div class="flex-1 min-w-0">
            <h4 class="font-semibold text-fabric-sepia text-sm truncate font-serif">{{ deletingBook.title }}</h4>
            <p class="text-xs text-fabric-thread/60 truncate">{{ deletingBook.author || '佚名' }}</p>
            <p class="text-xs text-fabric-thread/50 mt-1">{{ deletingBook.total_chapters }} 章节</p>
          </div>
        </div>
      </template>
    </ConfirmModal>
  </div>
</template>
