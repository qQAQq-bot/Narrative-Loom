<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useSettingsStore } from '@/stores/settings';
import { useTheme, type ThemeMode } from '@/composables/useTheme';
import { useTauri } from '@/composables/useTauri';
import ProviderList from '@/components/settings/ProviderList.vue';
import AgentList from '@/components/settings/AgentList.vue';
import TaskBindings from '@/components/settings/TaskBindings.vue';
import EmbeddingConfig from '@/components/settings/EmbeddingConfig.vue';
import PromptCards from '@/components/settings/PromptCards.vue';
import FabricSelect from '@/components/ui/FabricSelect.vue';
import FabricToggle from '@/components/ui/FabricToggle.vue';
import {
  AGENT_TYPES,
  THEME_OPTIONS,
  RETRY_COUNT_OPTIONS,
  AUTO_ACCEPT_OPTIONS,
} from '@/constants/labels';

const router = useRouter();
const settingsStore = useSettingsStore();
const { theme, setTheme } = useTheme();
const { invoke } = useTauri();

// Current section
const activeSection = ref('general');

// Library path state
const libraryPath = ref<string>('');
const isLoadingPath = ref(false);

// Logging state
const loggingEnabled = ref(false);
const isLoadingLogging = ref(false);

// Auto-accept state
const autoAcceptThreshold = ref<string>('off');
const isLoadingAutoAccept = ref(false);

// Enabled agents state
const enabledAgents = ref<string[]>([]);
const isLoadingEnabledAgents = ref(false);

// Request retry count state
const requestRetryCount = ref<string>('3');
const isLoadingRetryCount = ref(false);

// Navigation items
const navItems = [
  { id: 'general', label: '通用', icon: 'settings' },
  { id: 'providers', label: 'Provider', icon: 'cloud' },
  { id: 'agents', label: 'Agent', icon: 'cpu' },
  { id: 'prompt-cards', label: '提示词卡片', icon: 'card' },
  { id: 'tasks', label: '任务绑定', icon: 'link' },
  { id: 'embedding', label: 'Embedding', icon: 'vector' },
  { id: 'about', label: '关于', icon: 'info' },
];

function handleThemeChange(value: string) {
  setTheme(value as ThemeMode);
}

async function loadLibraryPath() {
  isLoadingPath.value = true;
  try {
    const path = await invoke<string>('get_library_path');
    libraryPath.value = path;
  } catch (e) {
    console.error('Failed to load library path:', e);
  } finally {
    isLoadingPath.value = false;
  }
}

async function selectLibraryPath() {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择书库目录',
    });

    if (selected && typeof selected === 'string') {
      await invoke('set_library_path', { path: selected });
      libraryPath.value = selected;
    }
  } catch (e) {
    console.error('Failed to select library path:', e);
  }
}

async function loadLoggingEnabled() {
  isLoadingLogging.value = true;
  try {
    const enabled = await invoke<boolean>('get_logging_enabled');
    loggingEnabled.value = enabled;
  } catch (e) {
    console.error('Failed to load logging setting:', e);
  } finally {
    isLoadingLogging.value = false;
  }
}

async function handleLoggingChange(value: boolean) {
  try {
    await invoke('set_logging_enabled', { enabled: value });
    loggingEnabled.value = value;
  } catch (e) {
    console.error('Failed to save logging setting:', e);
  }
}

async function loadAutoAcceptThreshold() {
  isLoadingAutoAccept.value = true;
  try {
    const threshold = await invoke<string>('get_auto_accept_threshold');
    autoAcceptThreshold.value = threshold;
  } catch (e) {
    console.error('Failed to load auto-accept setting:', e);
  } finally {
    isLoadingAutoAccept.value = false;
  }
}

async function handleAutoAcceptChange(value: string) {
  try {
    await invoke('set_auto_accept_threshold', { threshold: value });
    autoAcceptThreshold.value = value;
  } catch (e) {
    console.error('Failed to save auto-accept setting:', e);
  }
}

async function loadEnabledAgents() {
  isLoadingEnabledAgents.value = true;
  try {
    const agents = await invoke<string[]>('get_enabled_agents');
    enabledAgents.value = agents;
  } catch (e) {
    console.error('Failed to load enabled agents:', e);
  } finally {
    isLoadingEnabledAgents.value = false;
  }
}

function isAgentEnabled(agentType: string): boolean {
  return enabledAgents.value.includes(agentType);
}

async function toggleAgent(agentType: string) {
  try {
    let newAgents: string[];
    if (enabledAgents.value.includes(agentType)) {
      // Don't allow disabling all agents
      if (enabledAgents.value.length <= 1) {
        return;
      }
      newAgents = enabledAgents.value.filter(a => a !== agentType);
    } else {
      newAgents = [...enabledAgents.value, agentType];
    }
    await invoke('set_enabled_agents', { agents: newAgents });
    enabledAgents.value = newAgents;
  } catch (e) {
    console.error('Failed to save enabled agents:', e);
  }
}

async function loadRequestRetryCount() {
  isLoadingRetryCount.value = true;
  try {
    const count = await invoke<number>('get_request_retry_count');
    requestRetryCount.value = String(count);
  } catch (e) {
    console.error('Failed to load retry count:', e);
  } finally {
    isLoadingRetryCount.value = false;
  }
}

async function handleRetryCountChange(value: string) {
  const count = Number(value);
  if (!Number.isFinite(count)) return;
  try {
    await invoke('set_request_retry_count', { count });
    requestRetryCount.value = String(count);
  } catch (e) {
    console.error('Failed to save retry count:', e);
  }
}

function scrollToSection(sectionId: string) {
  activeSection.value = sectionId;
  const element = document.getElementById(`section-${sectionId}`);
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }
}

function goBack() {
  // Use router.back() to return to the previous page
  // If there's no history, go to the library
  if (window.history.length > 1) {
    router.back();
  } else {
    router.push('/');
  }
}

onMounted(() => {
  settingsStore.loadAll();
  loadLibraryPath();
  loadLoggingEnabled();
  loadAutoAcceptThreshold();
  loadEnabledAgents();
  loadRequestRetryCount();
});
</script>

<template>
  <div class="h-full flex bg-fabric-cream bg-weave">
    <!-- Left Sidebar Navigation with fabric styling -->
    <aside class="w-56 bg-fabric-warm border-r border-fabric-sand/40 shrink-0 flex flex-col">
      <!-- Header -->
      <div class="p-4 border-b border-fabric-sand/30">
        <button
          @click="goBack"
          class="flex items-center gap-2 text-fabric-thread hover:text-primary-600 transition-colors duration-220 group"
        >
          <svg class="w-5 h-5 group-hover:-translate-x-1 transition-transform duration-220" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
          </svg>
          <span class="text-sm font-medium">返回</span>
        </button>
      </div>

      <!-- Navigation -->
      <nav class="flex-1 p-3 space-y-1">
        <button
          v-for="item in navItems"
          :key="item.id"
          @click="scrollToSection(item.id)"
          :class="[
            'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-220',
            activeSection === item.id
              ? 'bg-primary-500/10 text-primary-700 dark:text-primary-300 shadow-fabric-inner'
              : 'text-fabric-thread hover:bg-fabric-sand/30 hover:text-fabric-sepia'
          ]"
        >
          <!-- Icons -->
          <svg v-if="item.icon === 'settings'" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          <svg v-else-if="item.icon === 'cloud'" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z" />
          </svg>
          <svg v-else-if="item.icon === 'cpu'" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
          </svg>
          <svg v-else-if="item.icon === 'link'" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
          </svg>
          <svg v-else-if="item.icon === 'vector'" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10 7h4M7 10v4M17 10v4M10 17h4" />
          </svg>
          <svg v-else-if="item.icon === 'card'" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
          </svg>
          <svg v-else-if="item.icon === 'info'" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span>{{ item.label }}</span>
        </button>
      </nav>

      <!-- Version info at bottom -->
      <div class="p-4 border-t border-fabric-sand/30">
        <div class="flex items-center gap-2.5">
          <!-- Loom/织布机图标 -->
          <div class="w-8 h-8 rounded-lg bg-fabric-sand/40 flex items-center justify-center">
            <svg class="w-4 h-4 text-fabric-sepia" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <!-- 织布机简化图标 -->
              <path d="M4 4h16v2H4zM4 18h16v2H4z" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M6 6v12M10 6v12M14 6v12M18 6v12" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="2 2"/>
              <path d="M4 11h16" stroke-linecap="round"/>
            </svg>
          </div>
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-fabric-sepia truncate">Narrative Loom</p>
            <p class="text-[10px] text-fabric-thread/40 font-mono">v0.1.0</p>
          </div>
        </div>
      </div>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 overflow-auto">
      <div class="max-w-3xl mx-auto p-6 space-y-6">
        <!-- Page Header -->
        <div class="mb-8">
          <h1 class="text-2xl font-bold text-fabric-sepia font-serif">设置</h1>
          <p class="text-fabric-thread/70 mt-1">配置应用程序、AI 服务和分析任务</p>
        </div>

        <!-- General Settings -->
        <section id="section-general" class="fabric-card stitch-border overflow-hidden">
          <div class="px-6 py-4 border-b border-fabric-sand/30 flex items-center gap-3">
            <div class="w-8 h-8 bg-fabric-sand/30 rounded-lg flex items-center justify-center">
              <svg class="w-4 h-4 text-fabric-thread" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-fabric-sepia">通用设置</h2>
              <p class="text-xs text-fabric-thread/60">外观和存储配置</p>
            </div>
          </div>

          <div class="p-6 space-y-6">
            <!-- Theme -->
            <div class="flex items-center justify-between">
              <div>
                <label class="block text-sm font-medium text-fabric-sepia">主题模式</label>
                <p class="text-xs text-fabric-thread/60 mt-0.5">选择应用的显示主题</p>
              </div>
              <div class="w-40">
                <FabricSelect
                  :model-value="theme"
                  @update:model-value="handleThemeChange"
                  :options="THEME_OPTIONS"
                  size="sm"
                />
              </div>
            </div>

            <div class="border-t border-fabric-sand/30"></div>

            <!-- Library Path -->
            <div>
              <label class="block text-sm font-medium text-fabric-sepia mb-1">书库路径</label>
              <p class="text-xs text-fabric-thread/60 mb-3">书籍数据将存储在此目录</p>
              <div class="flex gap-2">
                <input
                  type="text"
                  :value="libraryPath"
                  class="flex-1 px-3 py-2 border border-fabric-sand/50 rounded-lg bg-fabric-linen text-fabric-sepia text-sm"
                  placeholder="~/Documents/NarrativeLoom"
                  readonly
                />
                <button
                  @click="selectLibraryPath"
                  class="fabric-btn"
                >
                  选择目录
                </button>
              </div>
            </div>

            <div class="border-t border-fabric-sand/30"></div>

            <!-- API Logging -->
            <div class="flex items-center justify-between">
              <div>
                <label class="block text-sm font-medium text-fabric-sepia">API 日志记录</label>
                <p class="text-xs text-fabric-thread/60 mt-0.5">记录 AI 分析请求和响应，用于调试</p>
              </div>
              <FabricToggle
                :model-value="loggingEnabled"
                :disabled="isLoadingLogging"
                @update:model-value="handleLoggingChange"
              />
            </div>

            <div class="border-t border-fabric-sand/30"></div>

            <!-- Request retry count -->
            <div class="flex items-center justify-between">
              <div>
                <label class="block text-sm font-medium text-fabric-sepia">请求重试次数</label>
                <p class="text-xs text-fabric-thread/60 mt-0.5">AI 请求失败后的自动重试次数</p>
              </div>
              <div class="w-40">
                <FabricSelect
                  :model-value="requestRetryCount"
                  :disabled="isLoadingRetryCount"
                  @update:model-value="handleRetryCountChange"
                  :options="RETRY_COUNT_OPTIONS"
                  size="sm"
                />
              </div>
            </div>

            <div class="border-t border-fabric-sand/30"></div>

            <!-- Auto-accept threshold -->
            <div class="flex items-center justify-between">
              <div>
                <label class="block text-sm font-medium text-fabric-sepia">分析结果自动接受</label>
                <p class="text-xs text-fabric-thread/60 mt-0.5">根据置信度自动将卡片接受到故事圣经</p>
              </div>
              <div class="w-40">
                <FabricSelect
                  :model-value="autoAcceptThreshold"
                  @update:model-value="handleAutoAcceptChange"
                  :options="AUTO_ACCEPT_OPTIONS"
                  size="sm"
                />
              </div>
            </div>

            <div class="border-t border-fabric-sand/30"></div>

            <!-- Enabled Agents -->
            <div>
              <label class="block text-sm font-medium text-fabric-sepia mb-1">默认分析 Agent</label>
              <p class="text-xs text-fabric-thread/60 mb-3">选择章节分析时默认启用的 Agent 类型</p>
              <div class="flex flex-wrap gap-2">
                <button
                  v-for="agent in AGENT_TYPES"
                  :key="agent.type"
                  @click="toggleAgent(agent.type)"
                  :disabled="isLoadingEnabledAgents"
                  :aria-pressed="isAgentEnabled(agent.type)"
                  :aria-label="`${agent.label} Agent ${isAgentEnabled(agent.type) ? '已启用' : '已禁用'}`"
                  :class="[
                    'px-3 py-1.5 text-sm rounded-lg border transition-all duration-150',
                    isAgentEnabled(agent.type)
                      ? 'bg-primary-100 border-primary-300 text-primary-700 dark:bg-primary-900/30 dark:border-primary-600 dark:text-primary-300'
                      : 'bg-fabric-sand/20 border-fabric-sand/50 text-fabric-thread/60 hover:bg-fabric-sand/40',
                    enabledAgents.length <= 1 && isAgentEnabled(agent.type)
                      ? 'cursor-not-allowed opacity-70'
                      : ''
                  ]"
                  :title="enabledAgents.length <= 1 && isAgentEnabled(agent.type) ? '至少需要启用一个 Agent' : ''"
                >
                  <span class="flex items-center gap-1.5">
                    <svg v-if="isAgentEnabled(agent.type)" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                    </svg>
                    <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                    </svg>
                    {{ agent.label }}
                  </span>
                </button>
              </div>
            </div>
          </div>
        </section>

        <!-- Provider Settings -->
        <section id="section-providers" class="fabric-card stitch-border overflow-hidden">
          <div class="px-6 py-4 border-b border-fabric-sand/30 flex items-center gap-3">
            <div class="w-8 h-8 bg-accent-technique/10 rounded-lg flex items-center justify-center">
              <svg class="w-4 h-4 text-accent-technique" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-fabric-sepia">Provider 配置</h2>
              <p class="text-xs text-fabric-thread/60">配置 LLM API 提供者</p>
            </div>
          </div>
          <div class="p-6">
            <ProviderList />
          </div>
        </section>

        <!-- Agent Settings -->
        <section id="section-agents" class="fabric-card stitch-border overflow-hidden">
          <div class="px-6 py-4 border-b border-fabric-sand/30 flex items-center gap-3">
            <div class="w-8 h-8 bg-accent-setting/10 rounded-lg flex items-center justify-center">
              <svg class="w-4 h-4 text-accent-setting" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-fabric-sepia">Agent 配置</h2>
              <p class="text-xs text-fabric-thread/60">配置用于分析任务的 AI Agent</p>
            </div>
          </div>
          <div class="p-6">
            <AgentList />
          </div>
        </section>

        <!-- Prompt Cards -->
        <section id="section-prompt-cards" class="fabric-card stitch-border overflow-hidden">
          <div class="px-6 py-4 border-b border-fabric-sand/30 flex items-center gap-3">
            <div class="w-8 h-8 bg-primary-500/10 rounded-lg flex items-center justify-center">
              <svg class="w-4 h-4 text-primary-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-fabric-sepia">提示词卡片</h2>
              <p class="text-xs text-fabric-thread/60">全局系统提示词，自动注入到所有 Agent</p>
            </div>
          </div>
          <div class="p-6">
            <PromptCards />
          </div>
        </section>

        <!-- Task Bindings -->
        <section id="section-tasks" class="fabric-card stitch-border overflow-hidden">
          <div class="px-6 py-4 border-b border-fabric-sand/30 flex items-center gap-3">
            <div class="w-8 h-8 bg-accent-character/10 rounded-lg flex items-center justify-center">
              <svg class="w-4 h-4 text-accent-character" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-fabric-sepia">任务绑定</h2>
              <p class="text-xs text-fabric-thread/60">配置每种分析任务使用的 Agent</p>
            </div>
          </div>
          <div class="p-6">
            <TaskBindings />
          </div>
        </section>

        <!-- Embedding Configuration -->
        <section id="section-embedding" class="fabric-card stitch-border overflow-hidden">
          <div class="px-6 py-4 border-b border-fabric-sand/30 flex items-center gap-3">
            <div class="w-8 h-8 bg-accent-event/10 rounded-lg flex items-center justify-center">
              <svg class="w-4 h-4 text-accent-event" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 7h4M7 10v4M17 10v4M10 17h4" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-fabric-sepia">Embedding 配置</h2>
              <p class="text-xs text-fabric-thread/60">配置文本向量化服务，用于智能上下文检索</p>
            </div>
          </div>
          <div class="p-6">
            <EmbeddingConfig />
          </div>
        </section>

        <!-- About -->
        <section id="section-about" class="fabric-card stitch-border overflow-hidden">
          <div class="px-6 py-4 border-b border-fabric-sand/30 flex items-center gap-3">
            <div class="w-8 h-8 bg-fabric-sand/30 rounded-lg flex items-center justify-center">
              <svg class="w-4 h-4 text-fabric-thread" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-fabric-sepia">关于</h2>
              <p class="text-xs text-fabric-thread/60">应用信息</p>
            </div>
          </div>
          <div class="p-6">
            <div class="flex items-center gap-4 mb-6">
              <div class="w-16 h-16 bg-primary-500/90 rounded-2xl flex items-center justify-center shadow-fabric relative overflow-hidden">
                <!-- Stitch effect -->
                <div class="absolute inset-1 border border-dashed border-primary-300/30 rounded-xl pointer-events-none"></div>
                <svg class="w-8 h-8 text-white relative z-10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </div>
              <div>
                <h3 class="text-lg font-bold text-fabric-sepia font-serif">Narrative Loom</h3>
                <p class="text-sm text-fabric-thread/70">AI 小说写作辅助工具</p>
              </div>
            </div>

            <div class="space-y-3 text-sm">
              <div class="flex items-center justify-between py-2 border-b border-fabric-sand/30">
                <span class="text-fabric-thread/70">版本</span>
                <span class="font-mono text-fabric-sepia">0.1.0</span>
              </div>
              <div class="flex items-center justify-between py-2 border-b border-fabric-sand/30">
                <span class="text-fabric-thread/70">技术栈</span>
                <span class="text-fabric-sepia">Tauri + Vue 3 + Rust</span>
              </div>
              <div class="flex items-center justify-between py-2">
                <span class="text-fabric-thread/70">许可证</span>
                <span class="text-fabric-sepia">MIT</span>
              </div>
            </div>
          </div>
        </section>
      </div>
    </main>
  </div>
</template>
