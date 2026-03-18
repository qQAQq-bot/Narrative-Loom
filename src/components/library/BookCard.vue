<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { BookInfo } from '@/stores/library';
import Badge from '@/components/ui/Badge.vue';

const props = defineProps<{
  book: BookInfo;
}>();

const emit = defineEmits<{
  (e: 'click', book: BookInfo): void;
  (e: 'delete', book: BookInfo): void;
}>();

// Cover URL from backend (base64 data URL)
const coverUrl = ref<string | null>(null);

// Fetch cover from backend
async function fetchCover() {
  if (props.book.cover_path) {
    try {
      const dataUrl = await invoke<string | null>('get_book_cover', { bookId: props.book.id });
      coverUrl.value = dataUrl;
    } catch (e) {
      console.error('Failed to load cover:', e);
      coverUrl.value = null;
    }
  } else {
    coverUrl.value = null;
  }
}

// Fetch cover on mount and when book changes
onMounted(fetchCover);
watch(() => props.book.id, fetchCover);

// Calculate progress percentage
const progressPercent = computed(() => {
  if (props.book.total_chapters === 0) return 0;
  return Math.round((props.book.analyzed_chapters / props.book.total_chapters) * 100);
});
</script>

<template>
  <div
    class="group relative flex flex-col fabric-card overflow-hidden cursor-pointer transform transition-all duration-300 hover:scale-[1.02] hover:shadow-xl"
    @click="emit('click', book)"
  >
    <!-- Cover Area -->
    <div class="aspect-[3/4] w-full relative overflow-hidden">
      <!-- Cover Image (if available) -->
      <img
        v-if="coverUrl"
        :src="coverUrl"
        :alt="book.title"
        class="absolute inset-0 w-full h-full object-cover"
      />

      <!-- Fallback: Fabric texture pattern -->
      <template v-else>
        <div class="absolute inset-0 bg-gradient-to-br from-fabric-linen via-fabric-warm to-fabric-sand"></div>
        <!-- Fabric texture pattern -->
        <div class="absolute inset-0 bg-canvas opacity-30"></div>
        <!-- Decorative stitch pattern -->
        <div class="absolute inset-0 opacity-15 bg-[radial-gradient(circle_at_1px_1px,#8b7355_1px,transparent_0)] bg-[size:12px_12px]"></div>

        <!-- Book icon placeholder -->
        <div class="absolute inset-0 flex items-center justify-center">
          <div class="w-20 h-20 rounded-2xl bg-fabric-warm/60 backdrop-blur-sm flex items-center justify-center shadow-inner">
            <svg class="w-10 h-10 text-fabric-thread/40" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
            </svg>
          </div>
        </div>
      </template>

      <!-- Gradient overlay for better text visibility -->
      <div class="absolute inset-0 bg-gradient-to-t from-black/20 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300"></div>

      <!-- Delete Overlay -->
      <div class="absolute inset-0 bg-fabric-sepia/0 group-hover:bg-fabric-sepia/5 transition-colors duration-220">
        <button
          @click.stop="emit('delete', book)"
          class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-all duration-220
                 w-7 h-7 bg-fabric-warm/95 hover:bg-red-50 dark:hover:bg-red-900/50
                 text-fabric-thread hover:text-red-600 dark:hover:text-red-400
                 rounded-full shadow-fabric flex items-center justify-center transform scale-90 group-hover:scale-100"
          title="删除书籍"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3.5 h-3.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" />
          </svg>
        </button>
      </div>

      <!-- Status Badge -->
      <div class="absolute bottom-2 right-2">
        <Badge :variant="book.status" />
      </div>
    </div>

    <!-- Content -->
    <div class="p-3 flex flex-col flex-1 bg-fabric-warm">
      <h3 class="text-sm font-bold text-fabric-sepia leading-tight line-clamp-2 mb-1.5 group-hover:text-primary-600 transition-colors duration-220 font-serif">
        {{ book.title }}
      </h3>

      <div class="flex items-center justify-between mt-auto">
        <div class="flex items-center text-xs text-fabric-thread/70">
          <svg class="w-3 h-3 mr-1 opacity-60" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
          </svg>
          <span class="truncate max-w-[6rem]">{{ book.author || '佚名' }}</span>
        </div>

        <div class="text-xs font-medium text-fabric-thread/50 font-mono">
          {{ book.analyzed_chapters }}/{{ book.total_chapters }}
        </div>
      </div>

      <!-- Progress Bar with fabric styling -->
      <div class="mt-2 h-1 w-full bg-fabric-sand/40 rounded-full overflow-hidden">
        <div
          class="h-full transition-all duration-500"
          :class="{
            'bg-green-500': progressPercent === 100,
            'bg-primary-400': progressPercent > 0 && progressPercent < 100,
            'bg-fabric-thread/20': progressPercent === 0
          }"
          :style="{ width: `${progressPercent}%` }"
        ></div>
      </div>
    </div>
  </div>
</template>
