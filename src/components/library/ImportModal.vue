<script setup lang="ts">
import { ref } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import Modal from '@/components/ui/Modal.vue';
import Button from '@/components/ui/Button.vue';
import type { ImportPreview } from '@/stores/library';
import { useLibraryStore } from '@/stores/library';
import { useRouter } from 'vue-router';

const props = defineProps<{
  show: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'success'): void;
}>();

const store = useLibraryStore();
const router = useRouter();

const step = ref<'select' | 'preview' | 'embedding-required'>('select');
const loading = ref(false);
const error = ref<string | null>(null);
const importPath = ref('');
const preview = ref<ImportPreview | null>(null);
const embeddingStatus = ref<{ is_configured: boolean; message: string } | null>(null);

// Form data
const formData = ref({
  title: '',
  author: '',
});

const reset = () => {
  step.value = 'select';
  loading.value = false;
  error.value = null;
  importPath.value = '';
  preview.value = null;
  embeddingStatus.value = null;
  formData.value = { title: '', author: '' };
};

const handleClose = () => {
  // 允许关闭，即使在加载中也可以取消
  reset();
  emit('close');
};

const goToSettings = () => {
  handleClose();
  router.push('/settings');
};

const checkEmbeddingConfig = async (): Promise<boolean> => {
  try {
    const status = await invoke<{
      is_configured: boolean;
      provider: string;
      missing: string[];
      message: string;
    }>('check_embedding_configured');

    embeddingStatus.value = status;
    return status.is_configured;
  } catch (e) {
    console.error('Failed to check embedding config:', e);
    return false;
  }
};

const selectFile = async () => {
  try {
    // First check if embedding is configured
    const isConfigured = await checkEmbeddingConfig();
    if (!isConfigured) {
      step.value = 'embedding-required';
      return;
    }

    const selected = await open({
      multiple: false,
      filters: [
        { name: "Supported Files", extensions: ["txt", "epub"] },
        { name: "Text Files", extensions: ["txt"] },
        { name: "EPUB Files", extensions: ["epub"] },
      ],
    });

    if (selected && typeof selected === 'string') {
      importPath.value = selected;
      await loadPreview();
    }
  } catch (e) {
    error.value = '文件选择失败: ' + String(e);
  }
};

const loadPreview = async () => {
  loading.value = true;
  error.value = null;

  try {
    const result = await store.previewImport(importPath.value);
    preview.value = result;

    // Auto-fill form
    formData.value.title = result.title || '';
    formData.value.author = result.author || '';

    step.value = 'preview';
  } catch (e) {
    const errorMsg = String(e);
    console.error('Preview import failed:', e);
    error.value = `无法读取文件: ${errorMsg}`;
  } finally {
    loading.value = false;
  }
};

const handleImport = async () => {
  if (!importPath.value) return;

  // 立即关闭弹窗，让用户可以在书架上看到导入进度
  emit('success');
  handleClose();

  // 在后台进行导入
  try {
    await store.importBook(importPath.value, {
      title: formData.value.title || undefined,
      author: formData.value.author || undefined,
    });
  } catch (e) {
    console.error('导入失败:', e);
    // 可以考虑使用 toast 或其他方式通知用户导入失败
  }
};

const formatChars = (chars: number) => {
  return chars >= 10000 ? (chars / 10000).toFixed(1) + '万' : chars.toLocaleString();
};
</script>

<template>
  <Modal :show="show" title="导入书籍" @close="handleClose">
    <!-- Step: Embedding Required -->
    <div v-if="step === 'embedding-required'" class="py-8 text-center">
      <div class="mb-6 flex justify-center">
        <div class="w-24 h-24 bg-amber-50 dark:bg-amber-900/20 rounded-full flex items-center justify-center">
          <svg class="w-12 h-12 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        </div>
      </div>

      <h4 class="text-lg font-medium text-fabric-sepia mb-2">需要配置 Embedding 服务</h4>
      <p class="text-fabric-thread/70 mb-4 max-w-sm mx-auto">
        导入书籍前需要先配置 Embedding 服务，用于生成文本向量和智能搜索功能。
      </p>

      <div v-if="embeddingStatus" class="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-4 mb-6 mx-auto max-w-sm text-left">
        <p class="text-sm text-amber-700 dark:text-amber-300">
          {{ embeddingStatus.message }}
        </p>
      </div>

      <div class="flex justify-center gap-3">
        <Button variant="secondary" @click="handleClose">
          取消
        </Button>
        <Button @click="goToSettings">
          前往设置
        </Button>
      </div>
    </div>

    <!-- Step 1: Select File -->
    <div v-else-if="step === 'select'" class="py-8 text-center relative">
      <!-- Loading overlay for file parsing -->
      <div v-if="loading" class="absolute inset-0 bg-white/80 dark:bg-fabric-cream/80 backdrop-blur-sm z-10 flex flex-col items-center justify-center">
        <div class="animate-spin w-8 h-8 border-4 border-primary-600 border-t-transparent rounded-full mb-3"></div>
        <p class="text-sm text-fabric-thread">正在解析文件...</p>
      </div>

      <div class="mb-6 flex justify-center">
        <div class="w-24 h-24 bg-primary-50 dark:bg-primary-900/20 rounded-full flex items-center justify-center">
          <svg class="w-12 h-12 text-primary-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
          </svg>
        </div>
      </div>

      <h4 class="text-lg font-medium text-fabric-sepia mb-2">选择书籍文件</h4>
      <p class="text-fabric-thread/70 mb-8 max-w-sm mx-auto">
        支持 .txt 和 .epub 格式。AI 将自动分析章节结构。
      </p>

      <Button size="lg" @click="selectFile" :disabled="loading">
        选择文件
      </Button>

      <div v-if="error" class="mt-4 text-red-600 dark:text-red-400 text-sm bg-red-50 dark:bg-red-900/20 py-2 px-4 rounded-lg inline-block">
        {{ error }}
      </div>
    </div>

    <!-- Step 2: Preview & Metadata -->
    <div v-else class="space-y-6 relative">
      <div v-if="loading" class="absolute inset-0 bg-white/50 dark:bg-fabric-cream/50 backdrop-blur-sm z-10 flex items-center justify-center">
        <div class="animate-spin w-8 h-8 border-4 border-primary-600 border-t-transparent rounded-full"></div>
      </div>

      <!-- Stats Grid -->
      <div v-if="preview" class="grid grid-cols-3 gap-4 bg-fabric-linen/50 dark:bg-fabric-linen/30 rounded-xl p-4">
        <div class="text-center">
          <div class="text-2xl font-bold text-fabric-sepia font-serif">{{ preview.chapter_count }}</div>
          <div class="text-xs text-fabric-thread/60 uppercase tracking-wide">章节</div>
        </div>
        <div class="text-center border-l border-fabric-sand/40">
          <div class="text-2xl font-bold text-fabric-sepia font-serif">{{ formatChars(preview.total_chars) }}</div>
          <div class="text-xs text-fabric-thread/60 uppercase tracking-wide">字数</div>
        </div>
        <div class="text-center border-l border-fabric-sand/40">
          <div class="text-2xl font-bold text-fabric-sepia font-serif">{{ preview.encoding }}</div>
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
      <div v-if="preview" class="border border-fabric-sand/40 rounded-xl overflow-hidden">
        <div class="bg-fabric-linen/50 px-4 py-2 border-b border-fabric-sand/40 text-xs font-medium text-fabric-thread/70">
          目录预览 (前10章)
        </div>
        <div class="max-h-48 overflow-y-auto divide-y divide-fabric-sand/20">
          <div v-for="chapter in preview.chapters.slice(0, 10)" :key="chapter.index" class="px-4 py-2.5 flex justify-between items-center text-sm hover:bg-fabric-sand/10">
            <span class="font-medium text-fabric-sepia truncate max-w-[70%]">{{ chapter.title || `第 ${chapter.index} 章` }}</span>
            <span class="text-fabric-thread/50 text-xs font-mono">{{ chapter.char_count }}字</span>
          </div>
          <div v-if="preview.chapters.length > 10" class="px-4 py-2 text-center text-xs text-fabric-thread/50 bg-fabric-linen/30">
            ... 还有 {{ preview.chapters.length - 10 }} 章
          </div>
        </div>
      </div>

      <div v-if="error" class="text-red-600 dark:text-red-400 text-sm bg-red-50 dark:bg-red-900/20 p-3 rounded-lg flex items-center">
        <svg class="w-4 h-4 mr-2 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
          <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"></path>
        </svg>
        {{ error }}
      </div>
    </div>

    <template #footer>
      <div v-if="step === 'preview'" class="flex gap-3 w-full justify-end">
        <Button variant="secondary" @click="step = 'select'">
          重新选择
        </Button>
        <Button 
          variant="primary" 
          :loading="loading" 
          :disabled="!formData.title"
          @click="handleImport"
        >
          确认导入
        </Button>
      </div>
    </template>
  </Modal>
</template>
