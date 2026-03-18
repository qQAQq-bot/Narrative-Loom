<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import ExportModal from '@/components/export/ExportModal.vue';
import StyleExportModal from '@/components/export/StyleExportModal.vue';
import ConfirmDialog from '@/components/ui/ConfirmDialog.vue';
import BlueprintTimeline from '@/components/blueprint/BlueprintTimeline.vue';
import RelationshipGraph from '@/components/blueprint/RelationshipGraph.vue';
import EntityHistoryPanel from '@/components/bible/EntityHistoryPanel.vue';
import { formatRole, formatSettingType, formatImportance } from '@/constants/labels';
import { useBookStore } from '@/stores/book';

interface CharacterInfo {
  id: string;
  name: string;
  aliases: string[];
  description: string;
  role: string;
  first_appearance_chapter_id: string | null;
  traits: string[];
  relationships: any;
  evidence: string[];
  notes: string;
  updated_at: string;
}

interface SettingInfo {
  id: string;
  setting_type: string;
  name: string;
  description: string;
  properties: any;
  evidence: string[];
  notes: string;
  updated_at: string;
}

interface EventInfo {
  id: string;
  title: string;
  description: string;
  chapter_id: string;
  characters_involved: string[];
  importance: string;
  evidence: string[];
  notes: string;
  updated_at: string;
}

interface BibleStats {
  characters: number;
  settings: number;
  events: number;
}

const route = useRoute();
const router = useRouter();
const bookStore = useBookStore();
const bookId = computed(() => {
  const id = route.params.id;
  return typeof id === 'string' ? id : id[0];
});

const activeTab = ref<'characters' | 'settings' | 'events' | 'blueprint'>('characters');
const loading = ref(false);
const error = ref<string | null>(null);
const isDeleting = ref(false);

// Selected entity for history panel
const selectedEntity = ref<{
  id: string;
  name: string;
  type: 'character' | 'setting' | 'event';
} | null>(null);

const showHistoryPanel = computed(() => selectedEntity.value !== null);

// Blueprint view mode: 'timeline' or 'graph'
const blueprintViewMode = ref<'timeline' | 'graph'>('timeline');

// Component refs for refresh
const relationshipGraphRef = ref<{ refresh: () => void; focus?: (id: string) => void } | null>(null);

const characters = ref<CharacterInfo[]>([]);
const settings = ref<SettingInfo[]>([]);
const events = ref<EventInfo[]>([]);
const stats = ref<BibleStats>({
  characters: 0,
  settings: 0,
  events: 0
});

// 筛选状态
const characterFilter = ref<string | null>(null);
const settingFilter = ref<string | null>(null);
const eventFilter = ref<string | null>(null);

// 筛选选项
const characterFilters = [
  { key: null, label: '全部' },
  { key: 'protagonist', label: '主角' },
  { key: 'major', label: '重要配角' },
  { key: 'supporting', label: '配角' },
  { key: 'minor', label: '龙套' },
];

const settingFilters = [
  { key: null, label: '全部' },
  { key: 'location', label: '地点' },
  { key: 'organization', label: '组织' },
  { key: 'item', label: '物品' },
  { key: 'concept', label: '概念' },
  { key: 'power_system', label: '力量体系' },
  { key: 'era', label: '时代' },
  { key: 'worldview', label: '世界观' },
];

const eventFilters = [
  { key: null, label: '全部' },
  { key: 'critical', label: '关键' },
  { key: 'major', label: '重要' },
  { key: 'normal', label: '普通' },
  { key: 'minor', label: '次要' },
];

// 筛选后的数据
const filteredCharacters = computed(() => {
  if (!characterFilter.value) return characters.value;
  return characters.value.filter(c => c.role === characterFilter.value);
});

const filteredSettings = computed(() => {
  if (!settingFilter.value) return settings.value;
  return settings.value.filter(s => s.setting_type === settingFilter.value);
});

const filteredEvents = computed(() => {
  if (!eventFilter.value) return events.value;
  return events.value.filter(e => e.importance === eventFilter.value);
});

// 获取各筛选项的数量
const getCharacterFilterCount = (role: string | null) => {
  if (!role) return characters.value.length;
  return characters.value.filter(c => c.role === role).length;
};

const getSettingFilterCount = (type: string | null) => {
  if (!type) return settings.value.length;
  return settings.value.filter(s => s.setting_type === type).length;
};

const getEventFilterCount = (importance: string | null) => {
  if (!importance) return events.value.length;
  return events.value.filter(e => e.importance === importance).length;
};

// Delete confirmation state
const showDeleteConfirm = ref(false);
const deleteTarget = ref<{ type: 'character' | 'setting' | 'event'; id: string; name: string } | null>(null);

// Clear all confirmation state
const showClearConfirm = ref(false);
const clearTarget = ref<'characters' | 'settings' | 'events' | null>(null);
const isClearing = ref(false);

const tabs = [
  { id: 'characters', label: '人物' },
  { id: 'settings', label: '设定' },
  { id: 'events', label: '事件' },
  { id: 'blueprint', label: '蓝图' },
] as const;

const fetchData = async () => {
  if (!bookId.value) return;

  loading.value = true;
  error.value = null;

  try {
    // Load chapters if not already loaded
    if (bookStore.chapters.length === 0) {
      await bookStore.loadBook(bookId.value);
    }

    // Get book title and author from store
    if (bookStore.currentBook) {
      bookTitle.value = bookStore.currentBook.title;
      bookAuthor.value = bookStore.currentBook.author || '未知作者';
    }

    // Fetch stats
    stats.value = await invoke<BibleStats>('get_bible_stats', { bookId: bookId.value });

    // Fetch all data in parallel
    const [charsData, settingsData, eventsData] = await Promise.all([
      invoke<CharacterInfo[]>('get_characters', { bookId: bookId.value }),
      invoke<SettingInfo[]>('get_settings', { bookId: bookId.value }),
      invoke<EventInfo[]>('get_events', { bookId: bookId.value }),
    ]);

    characters.value = charsData || [];
    settings.value = settingsData || [];
    events.value = eventsData || [];

  } catch (e) {
    console.error('Failed to fetch bible data:', e);
    error.value = typeof e === 'string' ? e : '加载数据失败，请稍后重试';
  } finally {
    loading.value = false;
  }
};

onMounted(fetchData);

watch(() => route.params.id, fetchData);

// Delete handlers
function confirmDelete(type: 'character' | 'setting' | 'event', id: string, name: string) {
  deleteTarget.value = { type, id, name };
  showDeleteConfirm.value = true;
}

async function handleDelete() {
  if (!deleteTarget.value || !bookId.value) return;

  isDeleting.value = true;
  error.value = null;

  try {
    const { type, id } = deleteTarget.value;
    let deleted = false;

    switch (type) {
      case 'character':
        deleted = await invoke<boolean>('delete_character', { bookId: bookId.value, characterId: id });
        if (deleted) {
          characters.value = characters.value.filter(c => c.id !== id);
          stats.value.characters--;
          relationshipGraphRef.value?.refresh();
        }
        break;
      case 'setting':
        deleted = await invoke<boolean>('delete_setting', { bookId: bookId.value, settingId: id });
        if (deleted) {
          settings.value = settings.value.filter(s => s.id !== id);
          stats.value.settings--;
        }
        break;
      case 'event':
        deleted = await invoke<boolean>('delete_event', { bookId: bookId.value, eventId: id });
        if (deleted) {
          events.value = events.value.filter(e => e.id !== id);
          stats.value.events--;
        }
        break;
    }

    showDeleteConfirm.value = false;
    deleteTarget.value = null;
  } catch (e) {
    console.error('Failed to delete:', e);
    error.value = typeof e === 'string' ? e : '删除失败，请稍后重试';
  } finally {
    isDeleting.value = false;
  }
}

function cancelDelete() {
  showDeleteConfirm.value = false;
  deleteTarget.value = null;
}

// Clear all functions
function confirmClearAll(type: 'characters' | 'settings' | 'events') {
  clearTarget.value = type;
  showClearConfirm.value = true;
}

async function executeClearAll() {
  if (!clearTarget.value || !bookId.value) return;

  isClearing.value = true;
  try {
    const commandMap = {
      characters: 'clear_all_characters',
      settings: 'clear_all_settings',
      events: 'clear_all_events',
    };
    await invoke(commandMap[clearTarget.value], { bookId: bookId.value });

    // Clear local state
    if (clearTarget.value === 'characters') {
      characters.value = [];
      stats.value.characters = 0;
      relationshipGraphRef.value?.refresh();
    } else if (clearTarget.value === 'settings') {
      settings.value = [];
      stats.value.settings = 0;
    } else if (clearTarget.value === 'events') {
      events.value = [];
      stats.value.events = 0;
    }

    showClearConfirm.value = false;
    clearTarget.value = null;
  } catch (e) {
    console.error('Failed to clear:', e);
    error.value = typeof e === 'string' ? e : '清空失败，请稍后重试';
  } finally {
    isClearing.value = false;
  }
}

function cancelClear() {
  showClearConfirm.value = false;
  clearTarget.value = null;
}

const clearConfirmTitle = computed(() => {
  if (!clearTarget.value) return '';
  const typeMap = { characters: '人物', settings: '设定', events: '事件' };
  return `清空全部${typeMap[clearTarget.value]}`;
});

const clearConfirmMessage = computed(() => {
  if (!clearTarget.value) return '';
  const typeMap = { characters: '人物', settings: '设定', events: '事件' };
  const countMap = { characters: stats.value.characters, settings: stats.value.settings, events: stats.value.events };
  return `确定要清空全部 ${countMap[clearTarget.value]} 个${typeMap[clearTarget.value]}吗？此操作无法撤销。`;
});

function clearError() {
  error.value = null;
}

// Merge duplicates
const isMerging = ref(false);

async function mergeDuplicateCharacters() {
  if (!bookId.value) return;

  isMerging.value = true;
  try {
    const count = await invoke<number>('merge_duplicate_characters', { bookId: bookId.value });
    if (count > 0) {
      await fetchData();
    }
    return count;
  } catch (e) {
    console.error('Failed to merge characters:', e);
    error.value = typeof e === 'string' ? e : '合并失败';
  } finally {
    isMerging.value = false;
  }
}

async function mergeDuplicateSettings() {
  if (!bookId.value) return;

  isMerging.value = true;
  try {
    const count = await invoke<number>('merge_duplicate_settings', { bookId: bookId.value });
    if (count > 0) {
      await fetchData();
    }
    return count;
  } catch (e) {
    console.error('Failed to merge settings:', e);
    error.value = typeof e === 'string' ? e : '合并失败';
  } finally {
    isMerging.value = false;
  }
}

// Character role management
interface RoleUpdateResult {
  updated_count: number;
  protagonist_count: number;
  major_count: number;
  supporting_count: number;
  minor_count: number;
}

const isUpdatingRoles = ref(false);
const roleUpdateResult = ref<RoleUpdateResult | null>(null);

// Role editing state
const editingRoleCharId = ref<string | null>(null);

const roleOptions = [
  { value: 'protagonist', label: '主角' },
  { value: 'major', label: '重要配角' },
  { value: 'supporting', label: '配角' },
  { value: 'minor', label: '龙套' },
];

async function autoUpdateCharacterRoles() {
  if (!bookId.value) return;

  isUpdatingRoles.value = true;
  roleUpdateResult.value = null;
  try {
    const result = await invoke<RoleUpdateResult>('auto_update_character_roles', { bookId: bookId.value });
    roleUpdateResult.value = result;
    if (result.updated_count > 0) {
      await fetchData();
      // Refresh relationship graph if it exists
      relationshipGraphRef.value?.refresh();
    }
  } catch (e) {
    console.error('Failed to update roles:', e);
    error.value = typeof e === 'string' ? e : '更新角色失败';
  } finally {
    isUpdatingRoles.value = false;
  }
}

async function updateSingleCharacterRole(characterId: string, role: string) {
  if (!bookId.value) return;

  try {
    await invoke<boolean>('update_character_role', {
      bookId: bookId.value,
      characterId,
      role,
    });
    // Update local state
    const char = characters.value.find(c => c.id === characterId);
    if (char) {
      char.role = role;
    }
    // Refresh relationship graph if it exists
    relationshipGraphRef.value?.refresh();
  } catch (e) {
    console.error('Failed to update role:', e);
    error.value = typeof e === 'string' ? e : '更新角色失败';
  } finally {
    editingRoleCharId.value = null;
  }
}

function toggleRoleEdit(charId: string, event: Event) {
  event.stopPropagation();
  if (editingRoleCharId.value === charId) {
    editingRoleCharId.value = null;
  } else {
    editingRoleCharId.value = charId;
  }
}

// Helper to truncate text
const truncate = (text: string, length: number = 60) => {
  if (!text) return '';
  return text.length > length ? text.substring(0, length) + '...' : text;
};

// Formatting helpers
const getRoleBadgeClass = (role: string) => {
  switch (role) {
    case 'protagonist':
      return 'bg-amber-500/10 text-amber-600 border-amber-500/20 dark:text-amber-400';
    case 'major':
      return 'bg-purple-500/10 text-purple-600 border-purple-500/20 dark:text-purple-400';
    case 'supporting':
      return 'bg-accent-character/10 text-accent-character border-accent-character/20';
    case 'minor':
    default:
      return 'bg-gray-100 text-gray-500 border-gray-200 dark:bg-gray-700 dark:text-gray-400 dark:border-gray-600';
  }
};

const getTabClass = (tabId: string) => {
  if (activeTab.value === tabId) {
    if (tabId === 'characters') return 'bg-accent-character/10 text-accent-character border-accent-character/20 border shadow-sm';
    if (tabId === 'settings') return 'bg-accent-setting/10 text-accent-setting border-accent-setting/20 border shadow-sm';
    if (tabId === 'events') return 'bg-accent-event/10 text-accent-event border-accent-event/20 border shadow-sm';
    if (tabId === 'blueprint') return 'bg-blue-500/10 text-blue-600 border-blue-500/20 border shadow-sm dark:text-blue-400';
  }
  return 'bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-400 border-transparent hover:bg-gray-50 dark:hover:bg-gray-700 border hover:border-gray-200 dark:hover:border-gray-600';
};

const getDotColor = (tabId: string) => {
    if (tabId === 'characters') return 'bg-accent-character';
    if (tabId === 'settings') return 'bg-accent-setting';
    if (tabId === 'events') return 'bg-accent-event';
    if (tabId === 'blueprint') return 'bg-blue-500';
    return 'bg-gray-400';
};

// Get chapter title by ID
const getChapterTitle = (chapterId: string) => {
  const chapter = bookStore.chapters.find(c => c.id === chapterId);
  return chapter?.title || `章节 ${chapterId.slice(0, 8)}`;
};

const deleteConfirmTitle = computed(() => {
  if (!deleteTarget.value) return '';
  const typeMap = { character: '人物', setting: '设定', event: '事件' };
  return `删除${typeMap[deleteTarget.value.type]}`;
});

const deleteConfirmMessage = computed(() => {
  if (!deleteTarget.value) return '';
  return `确定要删除「${deleteTarget.value.name}」吗？此操作无法撤销。`;
});

// Export modal state
const showExportModal = ref(false);
const showStyleExportModal = ref(false);
const showExportMenu = ref(false);
const bookTitle = ref('书籍');
const bookAuthor = ref('未知作者');

const openExportModal = () => {
  showExportModal.value = true;
  showExportMenu.value = false;
};

const openStyleExportModal = () => {
  showStyleExportModal.value = true;
  showExportMenu.value = false;
};

const handleExported = (path: string) => {
  console.log('Exported to:', path);
};

// Entity history functions
const selectEntity = (id: string, name: string, type: 'character' | 'setting' | 'event') => {
  selectedEntity.value = { id, name, type };
};

const closeHistoryPanel = () => {
  selectedEntity.value = null;
};

const goToChapter = async (chapterId: string) => {
  // Navigate explicitly to the default child route so `<router-view />` is never empty.
  await router.push({ name: 'book-chapter', params: { id: bookId.value } });
  await bookStore.loadChapter(chapterId);
};
</script>

<template>
  <div class="h-full flex bg-gray-50/50 dark:bg-gray-900 overflow-hidden">
    <!-- Main Content -->
    <div class="flex-1 flex flex-col overflow-hidden">
    <!-- Header with Stats -->
    <div class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-8 py-5 flex-shrink-0 shadow-sm z-10">
      <div class="flex items-center justify-between mb-6">
        <div>
          <h2 class="text-2xl font-serif font-bold text-gray-900 dark:text-white tracking-tight">故事圣经</h2>
          <p class="text-xs text-gray-400 dark:text-gray-500 mt-1">Story Bible & World Building</p>
        </div>
        <div class="flex items-center gap-6">
          <!-- Export Dropdown -->
          <div class="relative">
            <button
              @click="showExportMenu = !showExportMenu"
              class="flex items-center gap-2 px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors shadow-sm"
            >
              <svg class="w-4 h-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
              </svg>
              导出
              <svg class="w-3 h-3 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
              </svg>
            </button>
            <!-- Dropdown Menu -->
            <div
              v-if="showExportMenu"
              class="absolute right-0 mt-2 w-56 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 py-1 z-20"
              @click.stop
            >
              <button
                @click="openExportModal"
                class="w-full flex items-center gap-3 px-4 py-2.5 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              >
                <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
                </svg>
                <div class="text-left">
                  <div class="font-medium">故事圣经</div>
                  <div class="text-xs text-gray-400 dark:text-gray-500">人物、设定、事件数据</div>
                </div>
              </button>
              <button
                @click="openStyleExportModal"
                class="w-full flex items-center gap-3 px-4 py-2.5 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              >
                <svg class="w-4 h-4 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                </svg>
                <div class="text-left">
                  <div class="font-medium">作者风格 Prompt</div>
                  <div class="text-xs text-gray-400 dark:text-gray-500">生成可独立使用的 AI Prompt</div>
                </div>
              </button>
            </div>
            <!-- Backdrop to close dropdown -->
            <div
              v-if="showExportMenu"
              class="fixed inset-0 z-10"
              @click="showExportMenu = false"
            ></div>
          </div>
          <!-- Stats -->
          <div class="flex gap-6 text-sm">
          <div class="flex flex-col items-center">
             <span class="text-xl font-bold text-accent-character">{{ stats.characters }}</span>
             <span class="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider">人物</span>
          </div>
          <div class="w-px h-8 bg-gray-100 dark:bg-gray-700"></div>
          <div class="flex flex-col items-center">
             <span class="text-xl font-bold text-accent-setting">{{ stats.settings }}</span>
             <span class="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider">设定</span>
          </div>
          <div class="w-px h-8 bg-gray-100 dark:bg-gray-700"></div>
          <div class="flex flex-col items-center">
             <span class="text-xl font-bold text-accent-event">{{ stats.events }}</span>
             <span class="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider">事件</span>
          </div>
        </div>
        </div>
      </div>

      <!-- Tabs -->
      <div class="flex gap-3">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          @click="activeTab = tab.id as any"
          class="px-5 py-2.5 text-sm font-medium rounded-xl transition-all duration-200 flex items-center gap-2"
          :class="getTabClass(tab.id)"
        >
          <span class="w-1.5 h-1.5 rounded-full transition-colors" :class="activeTab === tab.id ? getDotColor(tab.id) : 'bg-gray-300 dark:bg-gray-600'"></span>
          {{ tab.label }}
        </button>
      </div>
    </div>

    <!-- Error Banner -->
    <div
      v-if="error"
      class="mx-8 mt-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 px-4 py-3 rounded-xl flex items-center justify-between"
    >
      <div class="flex items-center gap-3">
        <svg class="w-5 h-5 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span class="text-sm">{{ error }}</span>
      </div>
      <button @click="clearError" class="text-red-500 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300">
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Content Area -->
    <div class="flex-1 min-h-0 overflow-y-auto p-8 relative">
      <div v-if="loading" class="flex flex-col justify-center items-center h-full text-gray-400 dark:text-gray-500 gap-3">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-400"></div>
        <span class="text-sm">读取世界观数据中...</span>
      </div>

      <div
        v-else
        :class="
          activeTab === 'blueprint'
            ? 'flex flex-col min-h-0 h-full w-full max-w-none'
            : 'max-w-7xl mx-auto'
        "
      >
        <!-- Characters Tab -->
        <div v-if="activeTab === 'characters'" class="space-y-6">
          <!-- Toolbar: filters (left) + actions (right) -->
          <div v-if="characters.length > 0" class="p-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl">
            <div class="grid grid-cols-1 md:grid-cols-[1fr_auto] gap-3 items-start">
              <!-- Filters -->
              <div class="flex items-center gap-2 flex-wrap">
                <svg class="w-4 h-4 text-gray-400 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
                </svg>
                <button
                  v-for="filter in characterFilters"
                  :key="filter.key ?? 'all'"
                  @click="characterFilter = filter.key"
                  :class="[
                    'px-3 py-1.5 text-xs font-medium rounded-lg border transition-all duration-200',
                    characterFilter === filter.key
                      ? 'bg-accent-character/10 text-accent-character border-accent-character/30 shadow-sm'
                      : 'bg-white dark:bg-gray-700 text-gray-600 dark:text-gray-400 border-gray-200 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600'
                  ]"
                >
                  {{ filter.label }}
                  <span class="ml-1.5 px-1.5 py-0.5 text-[10px] rounded-full bg-black/5 dark:bg-white/10">
                    {{ getCharacterFilterCount(filter.key) }}
                  </span>
                </button>
              </div>

              <!-- Actions -->
              <div class="flex justify-end gap-2 flex-wrap">
                <button
                  @click="autoUpdateCharacterRoles"
                  :disabled="isUpdatingRoles"
                  class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-amber-600 hover:text-amber-700 hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-900/20 rounded-lg transition-colors disabled:opacity-50"
                  title="基于出现频率自动更新人物角色"
                >
                  <svg v-if="isUpdatingRoles" class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                  <svg v-else class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
                  </svg>
                  智能更新角色
                </button>
                <button
                  @click="mergeDuplicateCharacters"
                  :disabled="isMerging"
                  class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-blue-600 hover:text-blue-700 hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-900/20 rounded-lg transition-colors disabled:opacity-50"
                >
                  <svg v-if="isMerging" class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                  <svg v-else class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                  </svg>
                  合并重复
                </button>
                <button
                  @click="confirmClearAll('characters')"
                  class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-red-600 hover:text-red-700 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                >
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                  清空全部
                </button>
              </div>
            </div>

            <!-- Role update result message -->
            <Transition
              enter-active-class="transition-all duration-300 ease-out"
              enter-from-class="opacity-0 -translate-y-1"
              enter-to-class="opacity-100 translate-y-0"
              leave-active-class="transition-all duration-200 ease-in"
              leave-from-class="opacity-100"
              leave-to-class="opacity-0"
            >
              <div v-if="roleUpdateResult" class="mt-3 flex items-center gap-2 px-3 py-1.5 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg">
                <svg class="w-4 h-4 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                </svg>
                <span class="text-xs text-green-700 dark:text-green-300">
                  已更新 <span class="font-semibold">{{ roleUpdateResult.updated_count }}</span> 个角色
                </span>
                <div class="flex items-center gap-1.5 ml-1">
                  <span v-if="roleUpdateResult.protagonist_count > 0" class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
                    {{ roleUpdateResult.protagonist_count }} 主角
                  </span>
                  <span v-if="roleUpdateResult.major_count > 0" class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400">
                    {{ roleUpdateResult.major_count }} 重要
                  </span>
                  <span v-if="roleUpdateResult.supporting_count > 0" class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-cyan-100 text-cyan-700 dark:bg-cyan-900/30 dark:text-cyan-400">
                    {{ roleUpdateResult.supporting_count }} 配角
                  </span>
                  <span v-if="roleUpdateResult.minor_count > 0" class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400">
                    {{ roleUpdateResult.minor_count }} 龙套
                  </span>
                </div>
                <button @click="roleUpdateResult = null" class="ml-1 p-0.5 text-green-400 hover:text-green-600 dark:hover:text-green-300 rounded">
                  <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </Transition>
          </div>
          <div v-if="characters.length === 0" class="flex flex-col items-center justify-center py-24 text-gray-400 dark:text-gray-500 border-2 border-dashed border-gray-200 dark:border-gray-700 rounded-2xl bg-white/50 dark:bg-gray-800/50">
            <svg class="w-12 h-12 mb-4 text-gray-300 dark:text-gray-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
            </svg>
            <p class="text-lg font-medium text-gray-500 dark:text-gray-400">暂无人物档案</p>
            <p class="text-sm mt-2">请在编辑器侧边栏通过AI助手分析章节生成人物卡</p>
          </div>
          <div v-else-if="filteredCharacters.length === 0" class="flex flex-col items-center justify-center py-24 text-gray-400 dark:text-gray-500 border-2 border-dashed border-gray-200 dark:border-gray-700 rounded-2xl bg-white/50 dark:bg-gray-800/50">
            <svg class="w-12 h-12 mb-4 text-gray-300 dark:text-gray-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
            </svg>
            <p class="text-lg font-medium text-gray-500 dark:text-gray-400">没有符合筛选条件的人物</p>
            <button @click="characterFilter = null" class="text-sm mt-2 text-accent-character hover:underline">清除筛选</button>
          </div>
          <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5">
            <div
              v-for="char in filteredCharacters"
              :key="char.id"
              :class="[
                'group bg-white dark:bg-gray-800 p-5 rounded-2xl shadow-sm border transition-all duration-300 relative',
                selectedEntity?.id === char.id
                  ? 'border-accent-character shadow-md ring-2 ring-accent-character/20'
                  : 'border-gray-100 dark:border-gray-700 hover:shadow-md hover:border-accent-character/30'
              ]"
            >
              <!-- Action Buttons (top right) -->
              <div class="absolute top-3 right-3 flex items-center gap-1">
                <button
                  @click.stop="selectEntity(char.id, char.name, 'character')"
                  class="p-1.5 rounded-lg text-gray-400 hover:text-accent-character hover:bg-accent-character/10 opacity-0 group-hover:opacity-100 transition-all"
                  title="查看出现历史"
                >
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                </button>
                <button
                  @click.stop="confirmDelete('character', char.id, char.name)"
                  class="p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 opacity-0 group-hover:opacity-100 transition-all"
                  title="删除"
                >
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
              <div class="flex justify-between items-start mb-3 pr-16">
                <h3 class="font-bold text-lg text-gray-900 dark:text-white group-hover:text-accent-character transition-colors">{{ char.name }}</h3>
                <!-- Role badge with edit dropdown -->
                <div class="relative">
                  <button
                    @click="toggleRoleEdit(char.id, $event)"
                    class="px-2.5 py-1 text-[10px] font-medium rounded-full border tracking-wide uppercase cursor-pointer hover:ring-2 hover:ring-offset-1 transition-all"
                    :class="getRoleBadgeClass(char.role)"
                    title="点击修改角色"
                  >
                    {{ formatRole(char.role) }}
                  </button>
                  <!-- Role edit dropdown -->
                  <div
                    v-if="editingRoleCharId === char.id"
                    class="absolute right-0 top-full mt-1 w-28 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 py-1 z-20"
                  >
                    <button
                      v-for="option in roleOptions"
                      :key="option.value"
                      @click.stop="updateSingleCharacterRole(char.id, option.value)"
                      class="w-full text-left px-3 py-1.5 text-xs hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                      :class="char.role === option.value ? 'font-medium text-accent-character' : 'text-gray-600 dark:text-gray-300'"
                    >
                      {{ option.label }}
                    </button>
                  </div>
                </div>
              </div>
              <!-- Close role dropdown when clicking outside -->
              <div
                v-if="editingRoleCharId === char.id"
                class="fixed inset-0 z-10"
                @click="editingRoleCharId = null"
              ></div>
              <div v-if="char.aliases && char.aliases.length" class="text-xs text-gray-400 dark:text-gray-500 mb-3 flex flex-wrap gap-1">
                <span v-for="alias in char.aliases.slice(0, 3)" :key="alias" class="bg-gray-50 dark:bg-gray-700 px-1.5 py-0.5 rounded text-gray-500 dark:text-gray-400">
                  {{ alias }}
                </span>
                <span v-if="char.aliases.length > 3" class="text-gray-300 dark:text-gray-600">+{{ char.aliases.length - 3 }}</span>
              </div>
              <p class="text-sm text-gray-600 dark:text-gray-300 leading-relaxed bg-gray-50/50 dark:bg-gray-700/50 p-3 rounded-lg max-h-32 overflow-y-auto scrollbar-thin scrollbar-thumb-gray-300 dark:scrollbar-thumb-gray-600 scrollbar-track-transparent">
                {{ char.description || '暂无描述信息...' }}
              </p>
              <div class="mt-4 pt-3 border-t border-gray-50 dark:border-gray-700 flex justify-between items-center text-xs text-gray-400 dark:text-gray-500">
                <span>ID: {{ char.id }}</span>
                <span>{{ new Date(char.updated_at).toLocaleDateString() }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Settings Tab -->
        <div v-if="activeTab === 'settings'" class="space-y-6">
          <!-- Toolbar: filters (left) + actions (right) -->
          <div v-if="settings.length > 0" class="p-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl">
            <div class="grid grid-cols-1 md:grid-cols-[1fr_auto] gap-3 items-start">
              <!-- Filters -->
              <div class="flex items-center gap-2 flex-wrap">
                <svg class="w-4 h-4 text-gray-400 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
                </svg>
                <button
                  v-for="filter in settingFilters"
                  :key="filter.key ?? 'all'"
                  @click="settingFilter = filter.key"
                  :class="[
                    'px-3 py-1.5 text-xs font-medium rounded-lg border transition-all duration-200',
                    settingFilter === filter.key
                      ? 'bg-accent-setting/10 text-accent-setting border-accent-setting/30 shadow-sm'
                      : 'bg-white dark:bg-gray-700 text-gray-600 dark:text-gray-400 border-gray-200 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600'
                  ]"
                >
                  {{ filter.label }}
                  <span class="ml-1.5 px-1.5 py-0.5 text-[10px] rounded-full bg-black/5 dark:bg-white/10">
                    {{ getSettingFilterCount(filter.key) }}
                  </span>
                </button>
              </div>

              <!-- Actions -->
              <div class="flex justify-end gap-2 flex-wrap">
                <button
                  @click="mergeDuplicateSettings"
                  :disabled="isMerging"
                  class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-blue-600 hover:text-blue-700 hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-900/20 rounded-lg transition-colors disabled:opacity-50"
                >
                  <svg v-if="isMerging" class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                  <svg v-else class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                  </svg>
                  合并重复
                </button>
                <button
                  @click="confirmClearAll('settings')"
                  class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-red-600 hover:text-red-700 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                >
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                  清空全部
                </button>
              </div>
            </div>
          </div>
          <div v-if="settings.length === 0" class="flex flex-col items-center justify-center py-24 text-gray-400 dark:text-gray-500 border-2 border-dashed border-gray-200 dark:border-gray-700 rounded-2xl bg-white/50 dark:bg-gray-800/50">
            <svg class="w-12 h-12 mb-4 text-gray-300 dark:text-gray-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
            </svg>
            <p class="text-lg font-medium text-gray-500 dark:text-gray-400">暂无世界设定</p>
            <p class="text-sm mt-2">当AI分析发现新的地点、物品或概念时会显示在这里</p>
          </div>
          <div v-else-if="filteredSettings.length === 0" class="flex flex-col items-center justify-center py-24 text-gray-400 dark:text-gray-500 border-2 border-dashed border-gray-200 dark:border-gray-700 rounded-2xl bg-white/50 dark:bg-gray-800/50">
            <svg class="w-12 h-12 mb-4 text-gray-300 dark:text-gray-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
            </svg>
            <p class="text-lg font-medium text-gray-500 dark:text-gray-400">没有符合筛选条件的设定</p>
            <button @click="settingFilter = null" class="text-sm mt-2 text-accent-setting hover:underline">清除筛选</button>
          </div>
          <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
            <div
              v-for="setting in filteredSettings"
              :key="setting.id"
              :class="[
                'group bg-white dark:bg-gray-800 p-5 rounded-2xl shadow-sm border transition-all duration-300 relative',
                selectedEntity?.id === setting.id
                  ? 'border-accent-setting shadow-md ring-2 ring-accent-setting/20'
                  : 'border-gray-100 dark:border-gray-700 hover:shadow-md hover:border-accent-setting/30'
              ]"
            >
              <!-- Action Buttons (top right) -->
              <div class="absolute top-3 right-3 flex items-center gap-1">
                <button
                  @click.stop="selectEntity(setting.id, setting.name, 'setting')"
                  class="p-1.5 rounded-lg text-gray-400 hover:text-accent-setting hover:bg-accent-setting/10 opacity-0 group-hover:opacity-100 transition-all"
                  title="查看出现历史"
                >
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                </button>
                <button
                  @click.stop="confirmDelete('setting', setting.id, setting.name)"
                  class="p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 opacity-0 group-hover:opacity-100 transition-all"
                  title="删除"
                >
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
              <div class="flex justify-between items-start mb-3 pr-16">
                <div class="flex items-center gap-2">
                  <div class="w-1.5 h-4 rounded-full bg-accent-setting/20 group-hover:bg-accent-setting transition-colors"></div>
                  <h3 class="font-bold text-lg text-gray-900 dark:text-white">{{ setting.name }}</h3>
                </div>
                <span class="px-2.5 py-0.5 text-[10px] font-bold rounded-md bg-gray-50 dark:bg-gray-700 text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  {{ formatSettingType(setting.setting_type) }}
                </span>
              </div>
              <p class="text-sm text-gray-600 dark:text-gray-300 leading-relaxed mb-3 max-h-24 overflow-y-auto scrollbar-thin scrollbar-thumb-gray-300 dark:scrollbar-thumb-gray-600 scrollbar-track-transparent">
                {{ setting.description || '暂无详细设定...' }}
              </p>
              <div class="flex flex-wrap gap-2">
                <span v-for="(val, key) in (typeof setting.properties === 'object' ? setting.properties : {})" :key="key"
                      class="text-[10px] px-2 py-1 bg-accent-setting/5 text-accent-setting rounded border border-accent-setting/10">
                  {{ key }}: {{ val }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <!-- Events Tab -->
        <div v-if="activeTab === 'events'" class="space-y-6">
          <!-- Toolbar: filters (left) + actions (right) -->
          <div v-if="events.length > 0" class="p-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl">
            <div class="grid grid-cols-1 md:grid-cols-[1fr_auto] gap-3 items-start">
              <!-- Filters -->
              <div class="flex items-center gap-2 flex-wrap">
                <svg class="w-4 h-4 text-gray-400 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
                </svg>
                <button
                  v-for="filter in eventFilters"
                  :key="filter.key ?? 'all'"
                  @click="eventFilter = filter.key"
                  :class="[
                    'px-3 py-1.5 text-xs font-medium rounded-lg border transition-all duration-200',
                    eventFilter === filter.key
                      ? 'bg-accent-event/10 text-accent-event border-accent-event/30 shadow-sm'
                      : 'bg-white dark:bg-gray-700 text-gray-600 dark:text-gray-400 border-gray-200 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600'
                  ]"
                >
                  {{ filter.label }}
                  <span class="ml-1.5 px-1.5 py-0.5 text-[10px] rounded-full bg-black/5 dark:bg-white/10">
                    {{ getEventFilterCount(filter.key) }}
                  </span>
                </button>
              </div>

              <!-- Actions -->
              <div class="flex justify-end">
                <button
                  @click="confirmClearAll('events')"
                  class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-red-600 hover:text-red-700 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                >
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                  清空全部
                </button>
              </div>
            </div>
          </div>
          <div v-if="events.length === 0" class="flex flex-col items-center justify-center py-24 text-gray-400 dark:text-gray-500 border-2 border-dashed border-gray-200 dark:border-gray-700 rounded-2xl bg-white/50 dark:bg-gray-800/50">
            <svg class="w-12 h-12 mb-4 text-gray-300 dark:text-gray-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            <p class="text-lg font-medium text-gray-500 dark:text-gray-400">暂无事件记录</p>
            <p class="text-sm mt-2">事件通常由AI从章节剧情中提取</p>
          </div>
          <div v-else-if="filteredEvents.length === 0" class="flex flex-col items-center justify-center py-24 text-gray-400 dark:text-gray-500 border-2 border-dashed border-gray-200 dark:border-gray-700 rounded-2xl bg-white/50 dark:bg-gray-800/50">
            <svg class="w-12 h-12 mb-4 text-gray-300 dark:text-gray-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
            </svg>
            <p class="text-lg font-medium text-gray-500 dark:text-gray-400">没有符合筛选条件的事件</p>
            <button @click="eventFilter = null" class="text-sm mt-2 text-accent-event hover:underline">清除筛选</button>
          </div>
          <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-5">
            <div
              v-for="event in filteredEvents"
              :key="event.id"
              :class="[
                'group bg-white dark:bg-gray-800 p-6 rounded-2xl shadow-sm border transition-all duration-300 relative',
                selectedEntity?.id === event.id
                  ? 'border-accent-event shadow-md ring-2 ring-accent-event/20'
                  : 'border-gray-100 dark:border-gray-700 hover:shadow-md hover:border-accent-event/30'
              ]"
            >
              <!-- Action Buttons (top right) -->
              <div class="absolute top-3 right-3 flex items-center gap-1">
                <button
                  @click.stop="selectEntity(event.id, event.title, 'event')"
                  class="p-1.5 rounded-lg text-gray-400 hover:text-accent-event hover:bg-accent-event/10 opacity-0 group-hover:opacity-100 transition-all"
                  title="查看出现历史"
                >
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                </button>
                <button
                  @click.stop="confirmDelete('event', event.id, event.title)"
                  class="p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 opacity-0 group-hover:opacity-100 transition-all"
                  title="删除"
                >
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
              <div class="flex justify-between items-start mb-4 pr-16">
                <h3 class="font-bold text-lg text-gray-900 dark:text-white group-hover:text-accent-event transition-colors">{{ event.title }}</h3>
                <span class="px-2.5 py-1 text-[10px] font-bold rounded-full bg-accent-event/10 text-accent-event border border-accent-event/20 uppercase tracking-wide">
                  {{ formatImportance(event.importance) }}
                </span>
              </div>

              <div class="flex items-center gap-2 text-xs text-gray-400 dark:text-gray-500 mb-4 bg-gray-50 dark:bg-gray-700 px-3 py-1.5 rounded-lg w-fit">
                <span class="font-medium text-gray-500 dark:text-gray-400">来源章节</span>
                <span>{{ getChapterTitle(event.chapter_id) }}</span>
              </div>

              <p class="text-sm text-gray-600 dark:text-gray-300 leading-relaxed mb-4">
                {{ truncate(event.description, 120) }}
              </p>

              <div v-if="event.characters_involved && event.characters_involved.length" class="flex -space-x-2 overflow-hidden py-1">
                 <div v-for="(char, idx) in event.characters_involved.slice(0,5)" :key="idx"
                      class="inline-flex items-center justify-center w-6 h-6 rounded-full bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 text-[10px] text-gray-500 dark:text-gray-400 shadow-sm"
                      :title="char">
                    {{ char.charAt(0) }}
                 </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Blueprint Tab -->
        <div v-if="activeTab === 'blueprint'" class="flex-1 min-h-0 -m-8 flex flex-col">
          <!-- View Content -->
          <div class="flex-1 overflow-hidden">
            <BlueprintTimeline
              v-if="blueprintViewMode === 'timeline'"
              :book-id="String(bookId)"
              v-model:view-mode="blueprintViewMode"
            />
            <RelationshipGraph
              v-else
              ref="relationshipGraphRef"
              :book-id="String(bookId)"
              v-model:view-mode="blueprintViewMode"
            />
          </div>
        </div>
      </div>
    </div>
    </div>

    <!-- Entity History Panel -->
    <Transition
      enter-active-class="transition-all duration-300 ease-out"
      enter-from-class="translate-x-full opacity-0"
      enter-to-class="translate-x-0 opacity-100"
      leave-active-class="transition-all duration-200 ease-in"
      leave-from-class="translate-x-0 opacity-100"
      leave-to-class="translate-x-full opacity-0"
    >
      <EntityHistoryPanel
        v-if="showHistoryPanel && selectedEntity"
        :book-id="String(bookId)"
        :entity-id="selectedEntity.id"
        :entity-name="selectedEntity.name"
        :entity-type="selectedEntity.type"
        @close="closeHistoryPanel"
        @go-to-chapter="goToChapter"
      />
    </Transition>

    <!-- Export Modal -->
    <ExportModal
      :book-id="String(bookId)"
      :book-title="bookTitle"
      :is-open="showExportModal"
      @close="showExportModal = false"
      @exported="handleExported"
    />

    <!-- Style Export Modal -->
    <StyleExportModal
      :book-id="String(bookId)"
      :book-title="bookTitle"
      :book-author="bookAuthor"
      :is-open="showStyleExportModal"
      @close="showStyleExportModal = false"
      @exported="handleExported"
    />

    <!-- Delete Confirmation Dialog -->
    <ConfirmDialog
      :show="showDeleteConfirm"
      :title="deleteConfirmTitle"
      :message="deleteConfirmMessage"
      confirm-text="删除"
      cancel-text="取消"
      variant="danger"
      :is-loading="isDeleting"
      @confirm="handleDelete"
      @cancel="cancelDelete"
    />

    <!-- Clear All Confirmation Dialog -->
    <ConfirmDialog
      :show="showClearConfirm"
      :title="clearConfirmTitle"
      :message="clearConfirmMessage"
      confirm-text="清空"
      cancel-text="取消"
      variant="danger"
      :is-loading="isClearing"
      @confirm="executeClearAll"
      @cancel="cancelClear"
    />
  </div>
</template>

<style scoped>
/* Refined Scrollbar */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background-color: rgba(156, 163, 175, 0.2);
  border-radius: 3px;
  transition: background-color 0.2s;
}
::-webkit-scrollbar-thumb:hover {
  background-color: rgba(156, 163, 175, 0.4);
}
</style>
