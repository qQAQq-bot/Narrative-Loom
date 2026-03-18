import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { useTauri } from '@/composables/useTauri';

export interface ProviderConfig {
  id: string;
  name: string;
  enabled: boolean;
  base_url: string;
  api_format: 'openai' | 'anthropic' | 'ollama' | 'chat_completions' | 'responses';
  path_override: string | null;
  api_key_ref: string;
  default_model: string;
  available_models: string[];
  headers: Record<string, string>;
  timeout_ms: number;
  max_retries: number;
}

export interface ProviderWithStatus extends ProviderConfig {
  has_api_key: boolean;
  masked_api_key: string | null;
}

export interface SaveProviderRequest extends ProviderConfig {
  api_key?: string;
}

export interface TestConnectionResult {
  success: boolean;
  message: string;
  latency_ms: number | null;
}

export type AgentKind =
  | { built_in: 'technique_analysis' | 'character_extraction' | 'setting_extraction' | 'event_extraction' | 'style_analysis' }
  | 'custom';

export type OutputMode =
  | 'text'
  | 'json_object'
  | { json_schema: { schema: unknown } };

export interface AgentConfig {
  id: string;
  name: string;
  kind: AgentKind;
  enabled: boolean;
  provider_id: string;
  model: string;
  temperature: number;
  max_tokens: number | null;
  system_prompt: string | null;
  output_mode: OutputMode;
}

export type TaskType =
  | 'technique_analysis'
  | 'character_extraction'
  | 'setting_extraction'
  | 'event_extraction'
  | 'style_analysis';

export interface TaskBindings {
  bindings: Record<TaskType, string>;
}

// Prompt Cards types
export type PromptCardPosition = 'prefix' | 'suffix';

export interface PromptCard {
  id: string;
  title: string;
  content: string;
  enabled: boolean;
  position: PromptCardPosition;
  order: number;
  updated_at?: string;
}

export const useSettingsStore = defineStore('settings', () => {
  const { invoke } = useTauri();

  // State
  const providers = ref<ProviderWithStatus[]>([]);
  const agents = ref<AgentConfig[]>([]);
  const taskBindings = ref<TaskBindings>({ bindings: {} as Record<TaskType, string> });
  const promptCards = ref<PromptCard[]>([]);
  const isLoadingPromptCards = ref(false);

  // Loading states
  const isLoadingProviders = ref(false);
  const isLoadingAgents = ref(false);
  const isSaving = ref(false);
  const isTesting = ref(false);
  const testingProviderId = ref<string | null>(null);

  // Error states
  const error = ref<string | null>(null);

  // Computed
  const enabledProviders = computed(() =>
    providers.value.filter(p => p.enabled)
  );

  const enabledAgents = computed(() =>
    agents.value.filter(a => a.enabled)
  );

  const enabledPromptCards = computed(() =>
    promptCards.value.filter(c => c.enabled)
  );

  const prefixCards = computed(() =>
    promptCards.value
      .filter(c => c.position === 'prefix')
      .sort((a, b) => a.order - b.order)
  );

  const suffixCards = computed(() =>
    promptCards.value
      .filter(c => c.position === 'suffix')
      .sort((a, b) => a.order - b.order)
  );

  // Actions
  async function loadProviders() {
    isLoadingProviders.value = true;
    error.value = null;

    try {
      providers.value = await invoke<ProviderWithStatus[]>('get_providers');
    } catch (e) {
      error.value = '加载 Provider 失败';
      console.error(e);
    } finally {
      isLoadingProviders.value = false;
    }
  }

  async function saveProvider(request: SaveProviderRequest) {
    isSaving.value = true;
    error.value = null;

    try {
      await invoke('save_provider', { request });
      await loadProviders();
    } catch (e) {
      error.value = '保存 Provider 失败';
      console.error(e);
      throw e;
    } finally {
      isSaving.value = false;
    }
  }

  async function deleteProvider(id: string) {
    isSaving.value = true;
    error.value = null;

    try {
      await invoke<boolean>('delete_provider', { id });
      await loadProviders();
    } catch (e) {
      error.value = '删除 Provider 失败';
      console.error(e);
      throw e;
    } finally {
      isSaving.value = false;
    }
  }

  async function testConnection(id: string): Promise<TestConnectionResult> {
    isTesting.value = true;
    testingProviderId.value = id;
    error.value = null;

    try {
      const result = await invoke<TestConnectionResult>('test_provider_connection', { id });
      return result;
    } catch (e) {
      error.value = '测试连接失败';
      console.error(e);
      throw e;
    } finally {
      isTesting.value = false;
      testingProviderId.value = null;
    }
  }

  async function loadAgents() {
    isLoadingAgents.value = true;
    error.value = null;

    try {
      agents.value = await invoke<AgentConfig[]>('get_agents');
    } catch (e) {
      error.value = '加载 Agent 失败';
      console.error(e);
    } finally {
      isLoadingAgents.value = false;
    }
  }

  async function saveAgent(agent: AgentConfig) {
    isSaving.value = true;
    error.value = null;

    try {
      await invoke('save_agent', { agent });
      await loadAgents();
    } catch (e) {
      error.value = '保存 Agent 失败';
      console.error(e);
      throw e;
    } finally {
      isSaving.value = false;
    }
  }

  async function deleteAgent(id: string) {
    isSaving.value = true;
    error.value = null;

    try {
      await invoke<boolean>('delete_agent', { id });
      await loadAgents();
    } catch (e) {
      error.value = '删除 Agent 失败';
      console.error(e);
      throw e;
    } finally {
      isSaving.value = false;
    }
  }

  async function loadTaskBindings() {
    try {
      taskBindings.value = await invoke<TaskBindings>('get_task_bindings');
    } catch (e) {
      console.error('加载 Task Bindings 失败', e);
    }
  }

  async function saveTaskBindings(bindings: TaskBindings) {
    isSaving.value = true;
    error.value = null;

    try {
      await invoke('save_task_bindings', { bindings });
      taskBindings.value = bindings;
    } catch (e) {
      error.value = '保存 Task Bindings 失败';
      console.error(e);
      throw e;
    } finally {
      isSaving.value = false;
    }
  }

  // Prompt Cards Actions
  async function loadPromptCards() {
    isLoadingPromptCards.value = true;
    error.value = null;

    try {
      promptCards.value = await invoke<PromptCard[]>('get_prompt_cards');
    } catch (e) {
      error.value = '加载提示词卡片失败';
      console.error(e);
    } finally {
      isLoadingPromptCards.value = false;
    }
  }

  async function savePromptCards(cards: PromptCard[]) {
    isSaving.value = true;
    error.value = null;

    try {
      // Normalize order values before saving
      const normalizedCards = normalizeCardOrders(cards);
      await invoke('save_prompt_cards', { cards: normalizedCards });
      promptCards.value = normalizedCards;
    } catch (e) {
      error.value = '保存提示词卡片失败';
      console.error(e);
      throw e;
    } finally {
      isSaving.value = false;
    }
  }

  function normalizeCardOrders(cards: PromptCard[]): PromptCard[] {
    // Separate by position and reassign order values
    const prefixList = cards
      .filter(c => c.position === 'prefix')
      .sort((a, b) => a.order - b.order);
    const suffixList = cards
      .filter(c => c.position === 'suffix')
      .sort((a, b) => a.order - b.order);

    prefixList.forEach((c, i) => c.order = i);
    suffixList.forEach((c, i) => c.order = i);

    return [...prefixList, ...suffixList];
  }

  async function upsertPromptCard(card: PromptCard) {
    const cards = [...promptCards.value];
    const existingIndex = cards.findIndex(c => c.id === card.id);

    if (existingIndex >= 0) {
      cards[existingIndex] = { ...card, updated_at: new Date().toISOString() };
    } else {
      // New card: assign order at the end of its position
      const samePositionCards = cards.filter(c => c.position === card.position);
      const maxOrder = samePositionCards.length > 0
        ? Math.max(...samePositionCards.map(c => c.order))
        : -1;
      cards.push({
        ...card,
        order: maxOrder + 1,
        updated_at: new Date().toISOString(),
      });
    }

    await savePromptCards(cards);
  }

  async function deletePromptCard(id: string) {
    const cards = promptCards.value.filter(c => c.id !== id);
    await savePromptCards(cards);
  }

  async function movePromptCard(id: string, direction: 'up' | 'down') {
    const cards = [...promptCards.value];
    const card = cards.find(c => c.id === id);
    if (!card) return;

    // Get cards in same position, sorted by order
    const samePositionCards = cards
      .filter(c => c.position === card.position)
      .sort((a, b) => a.order - b.order);

    const currentIndex = samePositionCards.findIndex(c => c.id === id);
    const targetIndex = direction === 'up' ? currentIndex - 1 : currentIndex + 1;

    if (targetIndex < 0 || targetIndex >= samePositionCards.length) return;

    // Swap orders
    const targetCard = samePositionCards[targetIndex];
    const tempOrder = card.order;
    card.order = targetCard.order;
    targetCard.order = tempOrder;

    await savePromptCards(cards);
  }

  async function togglePromptCard(id: string) {
    const cards = [...promptCards.value];
    const card = cards.find(c => c.id === id);
    if (!card) return;

    card.enabled = !card.enabled;
    card.updated_at = new Date().toISOString();

    await savePromptCards(cards);
  }

  async function loadAll() {
    await Promise.all([
      loadProviders(),
      loadAgents(),
      loadTaskBindings(),
      loadPromptCards(),
    ]);
  }

  function reset() {
    providers.value = [];
    agents.value = [];
    taskBindings.value = { bindings: {} as Record<TaskType, string> };
    promptCards.value = [];
    error.value = null;
  }

  return {
    // State
    providers,
    agents,
    taskBindings,
    promptCards,
    isLoadingProviders,
    isLoadingAgents,
    isLoadingPromptCards,
    isSaving,
    isTesting,
    testingProviderId,
    error,

    // Computed
    enabledProviders,
    enabledAgents,
    enabledPromptCards,
    prefixCards,
    suffixCards,

    // Actions
    loadProviders,
    saveProvider,
    deleteProvider,
    testConnection,
    loadAgents,
    saveAgent,
    deleteAgent,
    loadTaskBindings,
    saveTaskBindings,
    loadPromptCards,
    savePromptCards,
    upsertPromptCard,
    deletePromptCard,
    movePromptCard,
    togglePromptCard,
    loadAll,
    reset,
  };
});
