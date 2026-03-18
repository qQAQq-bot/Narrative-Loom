<script setup lang="ts">
import { ref, computed } from 'vue';
import { useRouter, useRoute } from 'vue-router';

const router = useRouter();
const route = useRoute();

const searchQuery = ref('');
const isFocused = ref(false);

const bookId = computed(() => route.params.id as string);

function handleSearch() {
  if (!searchQuery.value.trim() || !bookId.value) return;

  router.push({
    path: `/book/${bookId.value}/search`,
    query: { q: searchQuery.value.trim() },
  });
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    handleSearch();
  }
}

function navigateToSearch() {
  if (bookId.value) {
    router.push(`/book/${bookId.value}/search`);
  }
}
</script>

<template>
  <div class="relative">
    <div
      :class="[
        'flex items-center gap-2 px-3 py-2 rounded-lg border transition-all',
        isFocused
          ? 'border-primary-400 ring-2 ring-primary-100 bg-white'
          : 'border-gray-200 bg-gray-50 hover:bg-gray-100',
      ]"
    >
      <svg
        class="w-4 h-4 text-gray-400 shrink-0"
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 20 20"
        fill="currentColor"
      >
        <path
          fill-rule="evenodd"
          d="M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z"
          clip-rule="evenodd"
        />
      </svg>
      <input
        v-model="searchQuery"
        type="text"
        placeholder="搜索..."
        class="flex-1 bg-transparent border-none outline-none text-sm text-gray-700 placeholder-gray-400"
        @focus="isFocused = true"
        @blur="isFocused = false"
        @keydown="handleKeydown"
      />
      <button
        v-if="searchQuery"
        @click="searchQuery = ''"
        class="p-0.5 text-gray-400 hover:text-gray-600"
      >
        <svg class="w-4 h-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
        </svg>
      </button>
    </div>

    <!-- Quick access hint -->
    <div
      v-if="isFocused && !searchQuery"
      class="absolute top-full left-0 right-0 mt-1 p-2 bg-white border border-gray-200 rounded-lg shadow-lg text-xs text-gray-500"
    >
      <div class="flex items-center justify-between">
        <span>按 Enter 搜索</span>
        <button
          @mousedown.prevent="navigateToSearch"
          class="text-primary-600 hover:text-primary-700"
        >
          高级搜索
        </button>
      </div>
    </div>
  </div>
</template>
