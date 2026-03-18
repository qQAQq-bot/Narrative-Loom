<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  currentPage: number;
  totalItems: number;
  pageSize: number;
}>();

const emit = defineEmits<{
  (e: 'update:currentPage', value: number): void;
}>();

const totalPages = computed(() => Math.ceil(props.totalItems / props.pageSize));

const canGoPrev = computed(() => props.currentPage > 1);
const canGoNext = computed(() => props.currentPage < totalPages.value);

// Generate page numbers to display
const visiblePages = computed(() => {
  const pages: (number | 'ellipsis')[] = [];
  const total = totalPages.value;
  const current = props.currentPage;

  if (total <= 7) {
    // Show all pages if 7 or fewer
    for (let i = 1; i <= total; i++) {
      pages.push(i);
    }
  } else {
    // Always show first page
    pages.push(1);

    if (current > 3) {
      pages.push('ellipsis');
    }

    // Show pages around current
    const start = Math.max(2, current - 1);
    const end = Math.min(total - 1, current + 1);

    for (let i = start; i <= end; i++) {
      pages.push(i);
    }

    if (current < total - 2) {
      pages.push('ellipsis');
    }

    // Always show last page
    pages.push(total);
  }

  return pages;
});

function goToPage(page: number) {
  if (page >= 1 && page <= totalPages.value) {
    emit('update:currentPage', page);
  }
}

function goPrev() {
  if (canGoPrev.value) {
    emit('update:currentPage', props.currentPage - 1);
  }
}

function goNext() {
  if (canGoNext.value) {
    emit('update:currentPage', props.currentPage + 1);
  }
}
</script>

<template>
  <div v-if="totalPages > 1" class="flex items-center justify-center gap-1">
    <!-- Previous Button -->
    <button
      @click="goPrev"
      :disabled="!canGoPrev"
      class="p-2 rounded-lg transition-colors duration-180 disabled:opacity-40 disabled:cursor-not-allowed hover:bg-fabric-sand/30"
      :class="canGoPrev ? 'text-fabric-sepia' : 'text-fabric-thread/40'"
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
      </svg>
    </button>

    <!-- Page Numbers -->
    <template v-for="(page, index) in visiblePages" :key="index">
      <span v-if="page === 'ellipsis'" class="px-2 text-fabric-thread/40">...</span>
      <button
        v-else
        @click="goToPage(page)"
        class="min-w-[32px] h-8 px-2 rounded-lg text-sm font-medium transition-colors duration-180"
        :class="
          page === currentPage
            ? 'bg-primary-500 text-white'
            : 'text-fabric-sepia hover:bg-fabric-sand/30'
        "
      >
        {{ page }}
      </button>
    </template>

    <!-- Next Button -->
    <button
      @click="goNext"
      :disabled="!canGoNext"
      class="p-2 rounded-lg transition-colors duration-180 disabled:opacity-40 disabled:cursor-not-allowed hover:bg-fabric-sand/30"
      :class="canGoNext ? 'text-fabric-sepia' : 'text-fabric-thread/40'"
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
      </svg>
    </button>

    <!-- Page Info -->
    <span class="ml-3 text-sm text-fabric-thread/60">
      {{ currentPage }} / {{ totalPages }}
    </span>
  </div>
</template>
