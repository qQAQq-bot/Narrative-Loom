<script setup lang="ts">
import { ref, watch } from 'vue';
import Modal from '@/components/ui/Modal.vue';
import Button from '@/components/ui/Button.vue';
import type { ImportingBook } from '@/stores/library';

const props = defineProps<{
  show: boolean;
  book: ImportingBook | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'confirm', options: { title: string; author: string | null }): void;
  (e: 'reselect'): void;
}>();

// Form data
const formData = ref({
  title: '',
  author: '',
});

const loading = ref(false);

// Sync form data when book changes
watch(() => props.book, (book) => {
  if (book?.preview) {
    formData.value.title = book.preview.title || book.title;
    formData.value.author = book.preview.author || '';
  } else if (book) {
    formData.value.title = book.title;
    formData.value.author = book.author || '';
  }
}, { immediate: true });

const handleConfirm = async () => {
  loading.value = true;
  try {
    emit('confirm', {
      title: formData.value.title,
      author: formData.value.author || null,
    });
  } finally {
    loading.value = false;
  }
};

const handleReselect = () => {
  emit('reselect');
};

const formatChars = (chars: number) => {
  return chars >= 10000 ? (chars / 10000).toFixed(1) + '万' : chars.toLocaleString();
};
</script>

<template>
  <Modal :show="show" title="确认导入" @close="emit('close')">
    <div v-if="book?.preview" class="space-y-6">
      <!-- Stats Grid -->
      <div class="grid grid-cols-3 gap-4 bg-fabric-linen/50 dark:bg-fabric-linen/30 rounded-xl p-4">
        <div class="text-center">
          <div class="text-2xl font-bold text-fabric-sepia font-serif">{{ book.preview.chapter_count }}</div>
          <div class="text-xs text-fabric-thread/60 uppercase tracking-wide">章节</div>
        </div>
        <div class="text-center border-l border-fabric-sand/40">
          <div class="text-2xl font-bold text-fabric-sepia font-serif">{{ formatChars(book.preview.total_chars) }}</div>
          <div class="text-xs text-fabric-thread/60 uppercase tracking-wide">字数</div>
        </div>
        <div class="text-center border-l border-fabric-sand/40">
          <div class="text-2xl font-bold text-fabric-sepia font-serif">{{ book.preview.encoding }}</div>
          <div class="text-xs text-fabric-thread/60 uppercase tracking-wide">编码</div>
        </div>
      </div>

      <!-- Form -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium text-fabric-sepia mb-1.5">书名</label>
          <input
            v-model="formData.title"
            type="text"
            class="fabric-input"
            placeholder="输入书名"
          />
        </div>
        <div>
          <label class="block text-sm font-medium text-fabric-sepia mb-1.5">作者</label>
          <input
            v-model="formData.author"
            type="text"
            class="fabric-input"
            placeholder="输入作者（可选）"
          />
        </div>
      </div>

      <!-- Chapter Preview -->
      <div class="border border-fabric-sand/40 rounded-xl overflow-hidden">
        <div class="bg-fabric-linen/50 px-4 py-2 border-b border-fabric-sand/40 text-xs font-medium text-fabric-thread/70">
          目录预览 (前10章)
        </div>
        <div class="max-h-48 overflow-y-auto divide-y divide-fabric-sand/20">
          <div v-for="chapter in book.preview.chapters.slice(0, 10)" :key="chapter.index" class="px-4 py-2.5 flex justify-between items-center text-sm hover:bg-fabric-sand/10">
            <span class="font-medium text-fabric-sepia truncate max-w-[70%]">{{ chapter.title || `第 ${chapter.index} 章` }}</span>
            <span class="text-fabric-thread/50 text-xs font-mono">{{ chapter.char_count }}字</span>
          </div>
          <div v-if="book.preview.chapters.length > 10" class="px-4 py-2 text-center text-xs text-fabric-thread/50 bg-fabric-linen/30">
            ... 还有 {{ book.preview.chapters.length - 10 }} 章
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex gap-3 w-full justify-end">
        <Button variant="secondary" @click="handleReselect">
          重新选择
        </Button>
        <Button
          variant="primary"
          :loading="loading"
          :disabled="!formData.title"
          @click="handleConfirm"
        >
          确认导入
        </Button>
      </div>
    </template>
  </Modal>
</template>
