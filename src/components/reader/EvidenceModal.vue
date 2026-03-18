<script setup lang="ts">
import { computed } from 'vue';
import { useAnalysisStore } from '@/stores/analysis';
import { useBookStore } from '@/stores/book';

const analysisStore = useAnalysisStore();
const bookStore = useBookStore();

const isVisible = computed(() => analysisStore.showEvidenceModal);
const evidence = computed(() => analysisStore.activeEvidence);

// Find matching paragraph in chapter content
const matchResult = computed(() => {
  if (!evidence.value || !bookStore.currentChapter?.content) {
    return null;
  }

  const content = bookStore.currentChapter.content;
  const excerpt = evidence.value.excerpt;

  // Try to find the excerpt in the content
  const startIndex = content.indexOf(excerpt);
  if (startIndex === -1) {
    // Try fuzzy match - search for first 20 chars
    const searchStr = excerpt.slice(0, Math.min(20, excerpt.length));
    const fuzzyStart = content.indexOf(searchStr);
    if (fuzzyStart === -1) {
      return { found: false, excerpt };
    }
    return {
      found: true,
      startIndex: fuzzyStart,
      context: getContext(content, fuzzyStart, excerpt.length),
      excerpt,
    };
  }

  return {
    found: true,
    startIndex,
    context: getContext(content, startIndex, excerpt.length),
    excerpt,
  };
});

function getContext(content: string, startIndex: number, excerptLength: number): {
  before: string;
  highlight: string;
  after: string;
} {
  // Get 100 chars before and after
  const contextBefore = 100;
  const contextAfter = 100;

  const beforeStart = Math.max(0, startIndex - contextBefore);
  const afterEnd = Math.min(content.length, startIndex + excerptLength + contextAfter);

  const before = content.slice(beforeStart, startIndex);
  const highlight = content.slice(startIndex, startIndex + excerptLength);
  const after = content.slice(startIndex + excerptLength, afterEnd);

  return {
    before: (beforeStart > 0 ? '...' : '') + before,
    highlight,
    after: after + (afterEnd < content.length ? '...' : ''),
  };
}

function handleClose() {
  analysisStore.closeEvidenceModal();
}

function handleJumpToText() {
  // Emit event to scroll to the text in reader
  if (matchResult.value?.found && matchResult.value.startIndex !== undefined) {
    // The actual scrolling will be handled by ChapterReader
    // For now, just close the modal
    handleClose();
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="isVisible"
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
        @click.self="handleClose"
      >
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-black/50 backdrop-blur-sm"></div>

        <!-- Modal -->
        <div
          class="relative bg-white rounded-2xl shadow-2xl w-full max-w-xl max-h-[80vh] flex flex-col overflow-hidden"
        >
          <!-- Header -->
          <div class="flex items-center justify-between px-6 py-4 border-b border-gray-100">
            <h3 class="text-lg font-bold text-gray-900 flex items-center gap-2">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                class="h-5 w-5 text-yellow-500"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fill-rule="evenodd"
                  d="M15.621 4.379a3 3 0 00-4.242 0l-7 7a3 3 0 004.241 4.243h.001l.497-.5a.75.75 0 011.064 1.057l-.498.501-.002.002a4.5 4.5 0 01-6.364-6.364l7-7a4.5 4.5 0 016.368 6.36l-3.455 3.553A2.625 2.625 0 119.52 9.52l3.45-3.451a.75.75 0 111.061 1.06l-3.45 3.451a1.125 1.125 0 001.587 1.595l3.454-3.553a3 3 0 000-4.242z"
                  clip-rule="evenodd"
                />
              </svg>
              证据详情
            </h3>
            <button
              @click="handleClose"
              class="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                class="h-5 w-5"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fill-rule="evenodd"
                  d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                  clip-rule="evenodd"
                />
              </svg>
            </button>
          </div>

          <!-- Content -->
          <div class="flex-1 overflow-y-auto px-6 py-4">
            <div v-if="!evidence" class="text-center text-gray-400 py-8">
              没有选中的证据
            </div>

            <div v-else-if="matchResult?.found" class="space-y-4">
              <div class="text-sm text-gray-500 mb-2">
                在原文中找到匹配：
              </div>

              <div class="bg-gray-50 rounded-lg p-4 text-gray-700 leading-relaxed">
                <span class="text-gray-500">{{ matchResult.context.before }}</span>
                <mark class="bg-yellow-200 px-0.5 rounded">{{
                  matchResult.context.highlight
                }}</mark>
                <span class="text-gray-500">{{ matchResult.context.after }}</span>
              </div>

              <div class="border-t border-gray-100 pt-4">
                <div class="text-xs text-gray-400 mb-2">摘录原文：</div>
                <blockquote
                  class="pl-4 border-l-4 border-yellow-300 text-gray-600 italic"
                >
                  "{{ evidence.excerpt }}"
                </blockquote>
              </div>
            </div>

            <div v-else class="space-y-4">
              <div class="text-sm text-amber-600 bg-amber-50 px-4 py-3 rounded-lg">
                未在当前章节找到精确匹配，可能是跨章节引用或文本有细微差异。
              </div>

              <div class="border-t border-gray-100 pt-4">
                <div class="text-xs text-gray-400 mb-2">摘录原文：</div>
                <blockquote
                  class="pl-4 border-l-4 border-gray-300 text-gray-600 italic"
                >
                  "{{ evidence?.excerpt }}"
                </blockquote>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-gray-100 bg-gray-50">
            <button
              @click="handleClose"
              class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg transition-colors text-sm font-medium"
            >
              关闭
            </button>
            <button
              v-if="matchResult?.found"
              @click="handleJumpToText"
              class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors text-sm font-medium shadow-sm flex items-center gap-2"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                class="h-4 w-4"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fill-rule="evenodd"
                  d="M12.293 5.293a1 1 0 011.414 0l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414-1.414L14.586 11H3a1 1 0 110-2h11.586l-2.293-2.293a1 1 0 010-1.414z"
                  clip-rule="evenodd"
                />
              </svg>
              跳转到原文
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
  transition: all 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .relative,
.modal-leave-to .relative {
  transform: scale(0.95);
}
</style>
