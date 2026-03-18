<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useTauri } from '@/composables/useTauri';

const props = defineProps<{
  bookId: string;
  bookTitle: string;
  bookAuthor: string;
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'exported', path: string): void;
}>();

const { invoke } = useTauri();

// Tab state
type TabType = 'technique' | 'style';
const activeTab = ref<TabType>('technique');

// Style profile state
interface StyleProfileData {
  id: string;
  version: string;
  profile: unknown;
  analyzed_chapters: number;
  created_at: string;
  updated_at: string;
}
const styleProfile = ref<StyleProfileData | null>(null);
const isLoadingProfile = ref(false);

// Collected techniques count
const collectedTechniqueCount = ref(0);
const isLoadingTechniques = ref(false);

// Common export options
const exportFormat = ref<'markdown' | 'json'>('markdown');
const promptTemplate = ref<'generic' | 'openai' | 'claude' | 'local_llm'>('generic');

// Technique export options (Tab 1)
const includeNarrative = ref(true);
const includeDialogue = ref(true);
const includeDescription = ref(true);
const includePacing = ref(true);
const includeTension = ref(true);
const includeAtmosphere = ref(true);
const techniqueExampleCount = ref(3);

// Style export options (Tab 2)
const styleExampleCount = ref(3);

// State
const isExporting = ref(false);
const isPreviewLoading = ref(false);
const preview = ref('');
const error = ref<string | null>(null);
const showPreview = ref(false);
const copied = ref(false);
const exportSuccess = ref(false);
const exportedFilePath = ref<string | null>(null);

const hasStyleProfile = computed(() => styleProfile.value !== null);
const hasCollectedTechniques = computed(() => collectedTechniqueCount.value > 0);

const fileExtension = computed(() => {
  return exportFormat.value === 'markdown' ? '.md' : '.json';
});

const defaultFileName = computed(() => {
  const suffix = activeTab.value === 'technique' ? '技法Prompt' : '风格Prompt';
  return `${props.bookTitle}-${suffix}${fileExtension.value}`;
});

// Technique export options interface
interface TechniqueExportOptions {
  format: 'markdown' | 'json';
  include_narrative: boolean;
  include_dialogue: boolean;
  include_description: boolean;
  include_pacing: boolean;
  include_tension: boolean;
  include_atmosphere: boolean;
  example_count: number;
  prompt_template: string;
}

function getTechniqueExportOptions(): TechniqueExportOptions {
  return {
    format: exportFormat.value,
    include_narrative: includeNarrative.value,
    include_dialogue: includeDialogue.value,
    include_description: includeDescription.value,
    include_pacing: includePacing.value,
    include_tension: includeTension.value,
    include_atmosphere: includeAtmosphere.value,
    example_count: techniqueExampleCount.value,
    prompt_template: promptTemplate.value,
  };
}

// Style export options interface
interface StyleExportOptions {
  format: 'markdown' | 'json';
  example_count: number;
  prompt_template: string;
}

function getStyleExportOptions(): StyleExportOptions {
  return {
    format: exportFormat.value,
    example_count: styleExampleCount.value,
    prompt_template: promptTemplate.value,
  };
}

async function loadStyleProfile() {
  isLoadingProfile.value = true;
  try {
    styleProfile.value = await invoke<StyleProfileData | null>('get_style_profile', {
      bookId: props.bookId,
    });
  } catch (e) {
    console.error('Failed to load style profile:', e);
  } finally {
    isLoadingProfile.value = false;
  }
}

async function loadCollectedTechniques() {
  isLoadingTechniques.value = true;
  try {
    const count = await invoke<number>('get_collected_technique_count', {
      bookId: props.bookId,
    });
    collectedTechniqueCount.value = count;
  } catch (e) {
    console.error('Failed to load collected techniques:', e);
    collectedTechniqueCount.value = 0;
  } finally {
    isLoadingTechniques.value = false;
  }
}

async function loadPreview() {
  isPreviewLoading.value = true;
  error.value = null;

  try {
    if (activeTab.value === 'technique') {
      preview.value = await invoke<string>('get_technique_prompt_preview', {
        bookId: props.bookId,
        options: getTechniqueExportOptions(),
      });
    } else {
      preview.value = await invoke<string>('get_style_profile_prompt_preview', {
        bookId: props.bookId,
        options: getStyleExportOptions(),
      });
    }
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

    let result: { success: boolean; file_path: string | null; error: string | null };

    if (activeTab.value === 'technique') {
      result = await invoke<typeof result>('export_technique_prompt', {
        bookId: props.bookId,
        options: getTechniqueExportOptions(),
        outputPath: filePath,
      });
    } else {
      result = await invoke<typeof result>('export_style_profile_prompt', {
        bookId: props.bookId,
        options: getStyleExportOptions(),
        outputPath: filePath,
      });
    }

    if (result.success && result.file_path) {
      exportSuccess.value = true;
      exportedFilePath.value = result.file_path;
      emit('exported', result.file_path);
      // Close after showing success message
      setTimeout(() => {
        emit('close');
      }, 1500);
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

async function handleExportProfileJson() {
  isExporting.value = true;
  error.value = null;

  try {
    const { save } = await import('@tauri-apps/plugin-dialog');

    const filePath = await save({
      defaultPath: `${props.bookTitle}-风格档案.json`,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    });

    if (!filePath) {
      isExporting.value = false;
      return;
    }

    const result = await invoke<{
      success: boolean;
      file_path: string | null;
      error: string | null;
    }>('export_style_profile_json', {
      bookId: props.bookId,
      outputPath: filePath,
    });

    if (result.success && result.file_path) {
      exportSuccess.value = true;
      exportedFilePath.value = result.file_path;
      emit('exported', result.file_path);
    } else {
      error.value = result.error || '导出失败';
    }
  } catch (e) {
    error.value = String(e);
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
      copied.value = true;
      setTimeout(() => {
        copied.value = false;
      }, 2000);
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

function switchTab(tab: TabType) {
  activeTab.value = tab;
  preview.value = '';
  showPreview.value = false;
  error.value = null;
  exportSuccess.value = false;
  exportedFilePath.value = null;
}

// Reset preview when options change
watch(
  [exportFormat, promptTemplate, includeNarrative, includeDialogue, includeDescription, includePacing, includeTension, includeAtmosphere, techniqueExampleCount, styleExampleCount],
  () => {
    preview.value = '';
    showPreview.value = false;
  }
);

// Load data when modal opens
watch(() => props.isOpen, (open) => {
  if (open) {
    // Reset states
    error.value = null;
    exportSuccess.value = false;
    exportedFilePath.value = null;
    preview.value = '';
    showPreview.value = false;
    // Load data
    loadStyleProfile();
    loadCollectedTechniques();
  }
}, { immediate: true });
</script>

<template>
  <Teleport to="body">
    <div
      v-if="isOpen"
      class="fixed inset-0 z-50 flex items-center justify-center"
    >
      <!-- Backdrop -->
      <div
        class="absolute inset-0 bg-black/50 backdrop-blur-sm"
        @click="handleClose"
      ></div>

      <!-- Modal -->
      <div class="relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-3xl max-h-[90vh] flex flex-col overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700 shrink-0">
          <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white">导出写作风格</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
              基于《{{ bookTitle }}》的分析结果
            </p>
          </div>
          <button
            @click="handleClose"
            :disabled="isExporting"
            class="p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors disabled:opacity-50"
          >
            <svg class="w-5 h-5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
            </svg>
          </button>
        </div>

        <!-- Tabs -->
        <div class="flex border-b border-gray-200 dark:border-gray-700 px-6 shrink-0">
          <button
            @click="switchTab('technique')"
            :class="[
              'px-4 py-2.5 text-sm font-medium border-b-2 -mb-px transition-colors',
              activeTab === 'technique'
                ? 'border-purple-500 text-purple-600 dark:text-purple-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
            ]"
          >
            技法Prompt
          </button>
          <button
            @click="switchTab('style')"
            :class="[
              'px-4 py-2.5 text-sm font-medium border-b-2 -mb-px transition-colors',
              activeTab === 'style'
                ? 'border-purple-500 text-purple-600 dark:text-purple-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
            ]"
          >
            风格档案
          </button>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-6">
          <!-- Tab 1: 技法Prompt -->
          <template v-if="activeTab === 'technique'">
            <!-- Status -->
            <div v-if="!isLoadingTechniques" class="mb-4 p-4 rounded-lg" :class="hasCollectedTechniques ? 'bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800' : 'bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800'">
              <div class="flex items-start gap-3">
                <svg v-if="hasCollectedTechniques" class="w-5 h-5 text-green-500 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <svg v-else class="w-5 h-5 text-yellow-500 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
                <div class="flex-1">
                  <p v-if="hasCollectedTechniques" class="text-sm text-green-700 dark:text-green-300">
                    <span class="font-medium">已收藏 {{ collectedTechniqueCount }} 个技法</span>
                  </p>
                  <p v-else class="text-sm text-yellow-700 dark:text-yellow-300">
                    <span class="font-medium">暂无收藏的技法</span> - 请先在章节分析中收藏技法卡片
                  </p>
                </div>
              </div>
            </div>

            <div class="grid grid-cols-2 gap-6">
              <!-- Left Column: Options -->
              <div class="space-y-5">
                <!-- Format Selection -->
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">导出格式</label>
                  <div class="flex gap-4">
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        v-model="exportFormat"
                        type="radio"
                        value="markdown"
                        class="w-4 h-4 text-purple-600 border-gray-300 focus:ring-purple-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">Markdown</span>
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        v-model="exportFormat"
                        type="radio"
                        value="json"
                        class="w-4 h-4 text-purple-600 border-gray-300 focus:ring-purple-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">JSON</span>
                    </label>
                  </div>
                </div>

                <!-- Template Selection -->
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Prompt 模板</label>
                  <select
                    v-model="promptTemplate"
                    class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-sm text-gray-700 dark:text-gray-300 focus:ring-2 focus:ring-purple-500 focus:border-transparent"
                  >
                    <option value="generic">通用 System Prompt</option>
                    <option value="openai">OpenAI API 格式</option>
                    <option value="claude">Claude API 格式</option>
                    <option value="local_llm">本地 LLM (Ollama/llama.cpp)</option>
                  </select>
                </div>

                <!-- Example Count -->
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    每种技法的示例数量: {{ techniqueExampleCount }}
                  </label>
                  <input
                    v-model.number="techniqueExampleCount"
                    type="range"
                    min="1"
                    max="5"
                    class="w-full h-2 bg-gray-200 dark:bg-gray-600 rounded-lg appearance-none cursor-pointer accent-purple-600"
                  />
                  <div class="flex justify-between text-xs text-gray-400 mt-1">
                    <span>精简</span>
                    <span>详细</span>
                  </div>
                </div>

                <!-- Technique Categories -->
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">包含的技法类型</label>
                  <div class="grid grid-cols-2 gap-2">
                    <label class="flex items-center gap-2 cursor-pointer p-2 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">
                      <input
                        v-model="includeNarrative"
                        type="checkbox"
                        class="w-4 h-4 text-purple-600 border-gray-300 rounded focus:ring-purple-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">叙事技法</span>
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer p-2 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">
                      <input
                        v-model="includeDialogue"
                        type="checkbox"
                        class="w-4 h-4 text-purple-600 border-gray-300 rounded focus:ring-purple-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">对话技法</span>
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer p-2 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">
                      <input
                        v-model="includeDescription"
                        type="checkbox"
                        class="w-4 h-4 text-purple-600 border-gray-300 rounded focus:ring-purple-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">描写技法</span>
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer p-2 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">
                      <input
                        v-model="includePacing"
                        type="checkbox"
                        class="w-4 h-4 text-purple-600 border-gray-300 rounded focus:ring-purple-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">节奏技法</span>
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer p-2 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">
                      <input
                        v-model="includeTension"
                        type="checkbox"
                        class="w-4 h-4 text-purple-600 border-gray-300 rounded focus:ring-purple-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">张力技法</span>
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer p-2 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">
                      <input
                        v-model="includeAtmosphere"
                        type="checkbox"
                        class="w-4 h-4 text-purple-600 border-gray-300 rounded focus:ring-purple-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">氛围技法</span>
                    </label>
                  </div>
                </div>
              </div>

              <!-- Right Column: Preview -->
              <div class="flex flex-col">
                <div class="flex items-center justify-between mb-2">
                  <label class="text-sm font-medium text-gray-700 dark:text-gray-300">预览</label>
                  <button
                    @click="loadPreview"
                    :disabled="isPreviewLoading || !hasCollectedTechniques"
                    class="text-sm text-purple-600 hover:text-purple-700 dark:text-purple-400 dark:hover:text-purple-300 disabled:opacity-50 transition-colors"
                  >
                    {{ isPreviewLoading ? '加载中...' : showPreview ? '刷新' : '加载预览' }}
                  </button>
                </div>
                <div
                  v-if="showPreview"
                  class="flex-1 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-4 overflow-y-auto min-h-[300px] max-h-[400px]"
                >
                  <pre class="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap font-mono leading-relaxed">{{ preview.slice(0, 3000) }}{{ preview.length > 3000 ? '\n\n... (预览截断)' : '' }}</pre>
                </div>
                <div
                  v-else
                  class="flex-1 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-8 flex flex-col items-center justify-center text-center min-h-[300px]"
                >
                  <svg class="w-12 h-12 text-gray-300 dark:text-gray-600 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                  </svg>
                  <p class="text-gray-400 dark:text-gray-500 text-sm">
                    {{ hasCollectedTechniques ? '点击"加载预览"查看生成的 Prompt' : '请先收藏技法卡片' }}
                  </p>
                </div>
              </div>
            </div>
          </template>

          <!-- Tab 2: 风格档案 -->
          <template v-else>
            <!-- Status -->
            <div v-if="!isLoadingProfile" class="mb-4 p-4 rounded-lg" :class="hasStyleProfile ? 'bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800' : 'bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800'">
              <div class="flex items-start gap-3">
                <svg v-if="hasStyleProfile" class="w-5 h-5 text-green-500 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <svg v-else class="w-5 h-5 text-yellow-500 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
                <div class="flex-1">
                  <p v-if="hasStyleProfile" class="text-sm text-green-700 dark:text-green-300">
                    <span class="font-medium">已有风格档案</span> - 已分析 {{ styleProfile?.analyzed_chapters }} 章
                  </p>
                  <p v-else class="text-sm text-yellow-700 dark:text-yellow-300">
                    <span class="font-medium">暂无风格档案</span> - 请先对章节运行"风格分析"
                  </p>
                </div>
                <button
                  v-if="hasStyleProfile"
                  @click="handleExportProfileJson"
                  :disabled="isExporting"
                  class="px-3 py-1.5 text-xs bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 transition-colors"
                >
                  导出原始档案
                </button>
              </div>
            </div>

            <div class="grid grid-cols-2 gap-6">
              <!-- Left Column: Options (简化版) -->
              <div class="space-y-5">
                <!-- Format Selection -->
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">导出格式</label>
                  <div class="flex gap-4">
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        v-model="exportFormat"
                        type="radio"
                        value="markdown"
                        class="w-4 h-4 text-purple-600 border-gray-300 focus:ring-purple-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">Markdown</span>
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        v-model="exportFormat"
                        type="radio"
                        value="json"
                        class="w-4 h-4 text-purple-600 border-gray-300 focus:ring-purple-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">JSON</span>
                    </label>
                  </div>
                </div>

                <!-- Template Selection -->
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Prompt 模板</label>
                  <select
                    v-model="promptTemplate"
                    class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-sm text-gray-700 dark:text-gray-300 focus:ring-2 focus:ring-purple-500 focus:border-transparent"
                  >
                    <option value="generic">通用 System Prompt</option>
                    <option value="openai">OpenAI API 格式</option>
                    <option value="claude">Claude API 格式</option>
                    <option value="local_llm">本地 LLM (Ollama/llama.cpp)</option>
                  </select>
                </div>

                <!-- Example Count -->
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    示例段落数量: {{ styleExampleCount }}
                  </label>
                  <input
                    v-model.number="styleExampleCount"
                    type="range"
                    min="1"
                    max="5"
                    class="w-full h-2 bg-gray-200 dark:bg-gray-600 rounded-lg appearance-none cursor-pointer accent-purple-600"
                  />
                  <div class="flex justify-between text-xs text-gray-400 mt-1">
                    <span>精简</span>
                    <span>详细</span>
                  </div>
                </div>

                <!-- Info -->
                <div class="p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                  <div class="flex gap-3">
                    <svg class="w-5 h-5 text-blue-500 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    <div class="text-sm text-blue-700 dark:text-blue-300">
                      <p class="font-medium mb-1">风格档案包含</p>
                      <p class="text-blue-600 dark:text-blue-400 text-xs">
                        词汇特征、句式结构、叙事声音、对话风格、描写风格、节奏控制、情感表达、主题元素
                      </p>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Right Column: Preview -->
              <div class="flex flex-col">
                <div class="flex items-center justify-between mb-2">
                  <label class="text-sm font-medium text-gray-700 dark:text-gray-300">预览</label>
                  <button
                    @click="loadPreview"
                    :disabled="isPreviewLoading || !hasStyleProfile"
                    class="text-sm text-purple-600 hover:text-purple-700 dark:text-purple-400 dark:hover:text-purple-300 disabled:opacity-50 transition-colors"
                  >
                    {{ isPreviewLoading ? '加载中...' : showPreview ? '刷新' : '加载预览' }}
                  </button>
                </div>
                <div
                  v-if="showPreview"
                  class="flex-1 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-4 overflow-y-auto min-h-[300px] max-h-[400px]"
                >
                  <pre class="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap font-mono leading-relaxed">{{ preview.slice(0, 3000) }}{{ preview.length > 3000 ? '\n\n... (预览截断)' : '' }}</pre>
                </div>
                <div
                  v-else
                  class="flex-1 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-8 flex flex-col items-center justify-center text-center min-h-[300px]"
                >
                  <svg class="w-12 h-12 text-gray-300 dark:text-gray-600 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                  </svg>
                  <p class="text-gray-400 dark:text-gray-500 text-sm">
                    {{ hasStyleProfile ? '点击"加载预览"查看生成的 Prompt' : '请先运行风格分析' }}
                  </p>
                </div>
              </div>
            </div>
          </template>

          <!-- Error -->
          <div v-if="error" class="mt-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-600 dark:text-red-400 text-sm">
            {{ error }}
          </div>

          <!-- Success -->
          <div v-if="exportSuccess" class="mt-4 p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg">
            <div class="flex items-center gap-3">
              <svg class="w-6 h-6 text-green-500 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <div class="flex-1">
                <p class="text-green-700 dark:text-green-300 font-medium">导出成功</p>
                <p class="text-green-600 dark:text-green-400 text-sm mt-0.5 break-all">{{ exportedFilePath }}</p>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex items-center justify-between px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50 shrink-0">
          <button
            @click="copyToClipboard"
            :disabled="(activeTab === 'technique' && !hasCollectedTechniques) || (activeTab === 'style' && !hasStyleProfile)"
            class="flex items-center gap-2 px-4 py-2 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-200 dark:hover:bg-gray-700 rounded-lg transition-colors text-sm disabled:opacity-50"
          >
            <svg v-if="copied" class="w-4 h-4 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
            <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
            <span>{{ copied ? '已复制' : '复制到剪贴板' }}</span>
          </button>
          <div class="flex gap-3">
            <button
              @click="handleClose"
              :disabled="isExporting"
              class="px-4 py-2 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700 rounded-lg transition-colors disabled:opacity-50"
            >
              取消
            </button>
            <button
              @click="handleExport"
              :disabled="isExporting || (activeTab === 'technique' && !hasCollectedTechniques) || (activeTab === 'style' && !hasStyleProfile)"
              class="px-6 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700 disabled:opacity-50 transition-colors font-medium flex items-center gap-2"
            >
              <svg v-if="isExporting" class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span>{{ isExporting ? '导出中...' : '导出文件' }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
