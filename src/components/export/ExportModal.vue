<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useTauri } from '@/composables/useTauri';

const props = defineProps<{
  bookId: string;
  bookTitle: string;
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'exported', path: string): void;
}>();

const { invoke } = useTauri();

// Export options
const exportFormat = ref<'markdown' | 'json'>('markdown');
const includeCharacters = ref(true);
const includeSettings = ref(true);
const includeEvents = ref(true);
const includeTechniques = ref(true);

// State
const isExporting = ref(false);
const isPreviewLoading = ref(false);
const preview = ref('');
const error = ref<string | null>(null);
const showPreview = ref(false);

const fileExtension = computed(() => {
  return exportFormat.value === 'markdown' ? '.md' : '.json';
});

const defaultFileName = computed(() => {
  return `${props.bookTitle}-故事圣经${fileExtension.value}`;
});

interface ExportOptions {
  format: 'markdown' | 'json';
  include_characters: boolean;
  include_settings: boolean;
  include_events: boolean;
  include_techniques: boolean;
}

function getExportOptions(): ExportOptions {
  return {
    format: exportFormat.value,
    include_characters: includeCharacters.value,
    include_settings: includeSettings.value,
    include_events: includeEvents.value,
    include_techniques: includeTechniques.value,
  };
}

async function loadPreview() {
  isPreviewLoading.value = true;
  error.value = null;

  try {
    preview.value = await invoke<string>('get_export_preview', {
      bookId: props.bookId,
      options: getExportOptions(),
    });
    showPreview.value = true;
  } catch (e) {
    error.value = '加载预览失败';
    console.error(e);
  } finally {
    isPreviewLoading.value = false;
  }
}

async function handleExport() {
  isExporting.value = true;
  error.value = null;

  try {
    // Use Tauri's save dialog
    const { save } = await import('@tauri-apps/plugin-dialog');

    const filePath = await save({
      defaultPath: defaultFileName.value,
      filters: [
        {
          name: exportFormat.value === 'markdown' ? 'Markdown' : 'JSON',
          extensions: [exportFormat.value === 'markdown' ? 'md' : 'json'],
        },
      ],
    });

    if (!filePath) {
      isExporting.value = false;
      return;
    }

    const result = await invoke<{
      success: boolean;
      file_path: string | null;
      error: string | null;
    }>('export_bible', {
      bookId: props.bookId,
      options: getExportOptions(),
      outputPath: filePath,
    });

    if (result.success && result.file_path) {
      emit('exported', result.file_path);
      emit('close');
    } else {
      error.value = result.error || '导出失败';
    }
  } catch (e) {
    error.value = '导出失败';
    console.error(e);
  } finally {
    isExporting.value = false;
  }
}

async function copyToClipboard() {
  if (!preview.value) {
    await loadPreview();
  }

  if (preview.value) {
    try {
      await navigator.clipboard.writeText(preview.value);
      // Could add a toast notification here
    } catch (e) {
      console.error('Failed to copy to clipboard', e);
    }
  }
}

function handleClose() {
  if (!isExporting.value) {
    emit('close');
  }
}

// Reset preview when options change
watch([exportFormat, includeCharacters, includeSettings, includeEvents, includeTechniques], () => {
  preview.value = '';
  showPreview.value = false;
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="isOpen"
      class="fixed inset-0 z-50 flex items-center justify-center"
    >
      <!-- Backdrop -->
      <div
        class="absolute inset-0 bg-black/50"
        @click="handleClose"
      ></div>

      <!-- Modal -->
      <div class="relative bg-white rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] flex flex-col overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 shrink-0">
          <h2 class="text-lg font-semibold text-gray-900">导出故事圣经</h2>
          <button
            @click="handleClose"
            :disabled="isExporting"
            class="p-1 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-gray-100 transition-colors disabled:opacity-50"
          >
            <svg class="w-5 h-5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
            </svg>
          </button>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-6">
          <!-- Format Selection -->
          <div class="mb-6">
            <label class="block text-sm font-medium text-gray-700 mb-2">导出格式</label>
            <div class="flex gap-4">
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  v-model="exportFormat"
                  type="radio"
                  value="markdown"
                  class="w-4 h-4 text-primary-600 border-gray-300 focus:ring-primary-500"
                />
                <span class="text-sm text-gray-700">Markdown (.md)</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  v-model="exportFormat"
                  type="radio"
                  value="json"
                  class="w-4 h-4 text-primary-600 border-gray-300 focus:ring-primary-500"
                />
                <span class="text-sm text-gray-700">JSON (.json)</span>
              </label>
            </div>
          </div>

          <!-- Include Options -->
          <div class="mb-6">
            <label class="block text-sm font-medium text-gray-700 mb-2">包含内容</label>
            <div class="grid grid-cols-2 gap-3">
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  v-model="includeCharacters"
                  type="checkbox"
                  class="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                />
                <span class="text-sm text-gray-700">人物</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  v-model="includeSettings"
                  type="checkbox"
                  class="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                />
                <span class="text-sm text-gray-700">设定</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  v-model="includeEvents"
                  type="checkbox"
                  class="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                />
                <span class="text-sm text-gray-700">事件</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  v-model="includeTechniques"
                  type="checkbox"
                  class="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                />
                <span class="text-sm text-gray-700">收藏的技法</span>
              </label>
            </div>
          </div>

          <!-- Preview Section -->
          <div class="mb-4">
            <div class="flex items-center justify-between mb-2">
              <label class="block text-sm font-medium text-gray-700">预览</label>
              <button
                @click="loadPreview"
                :disabled="isPreviewLoading"
                class="text-sm text-primary-600 hover:text-primary-700 disabled:opacity-50"
              >
                {{ isPreviewLoading ? '加载中...' : showPreview ? '刷新预览' : '加载预览' }}
              </button>
            </div>
            <div
              v-if="showPreview"
              class="bg-gray-50 border border-gray-200 rounded-lg p-4 max-h-64 overflow-y-auto"
            >
              <pre class="text-xs text-gray-700 whitespace-pre-wrap font-mono">{{ preview.slice(0, 2000) }}{{ preview.length > 2000 ? '\n\n... (预览截断)' : '' }}</pre>
            </div>
            <div
              v-else
              class="bg-gray-50 border border-gray-200 rounded-lg p-8 text-center text-gray-400"
            >
              点击"加载预览"查看导出内容
            </div>
          </div>

          <!-- Error -->
          <div v-if="error" class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-600 text-sm">
            {{ error }}
          </div>
        </div>

        <!-- Footer -->
        <div class="flex items-center justify-between px-6 py-4 border-t border-gray-200 bg-gray-50 shrink-0">
          <button
            @click="copyToClipboard"
            class="px-4 py-2 text-gray-600 hover:text-gray-900 hover:bg-gray-200 rounded-lg transition-colors text-sm"
          >
            复制到剪贴板
          </button>
          <div class="flex gap-3">
            <button
              @click="handleClose"
              :disabled="isExporting"
              class="px-4 py-2 text-gray-600 hover:bg-gray-200 rounded-lg transition-colors disabled:opacity-50"
            >
              取消
            </button>
            <button
              @click="handleExport"
              :disabled="isExporting"
              class="px-6 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 transition-colors font-medium"
            >
              <span v-if="isExporting">导出中...</span>
              <span v-else>导出文件</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
