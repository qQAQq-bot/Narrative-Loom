import { ref, computed, readonly } from 'vue';
import { useTauri } from '@/composables/useTauri';
import type {
  ProviderWithStatus,
  SaveProviderRequest,
  TestConnectionResult,
} from '@/stores/settings';

/**
 * Composable for managing LLM providers
 * Provides a convenient API for provider operations with loading states
 */
export function useProviders() {
  const { invoke } = useTauri();

  // State
  const providers = ref<ProviderWithStatus[]>([]);
  const isLoading = ref(false);
  const isSaving = ref(false);
  const error = ref<string | null>(null);

  // Testing state
  const testingProviderId = ref<string | null>(null);
  const testResult = ref<TestConnectionResult | null>(null);

  // Computed
  const enabledProviders = computed(() =>
    providers.value.filter((p) => p.enabled)
  );

  const providerCount = computed(() => providers.value.length);

  const hasProviders = computed(() => providers.value.length > 0);

  /**
   * Get provider by ID
   */
  function getProvider(id: string): ProviderWithStatus | undefined {
    return providers.value.find((p) => p.id === id);
  }

  /**
   * Get provider by name (case-insensitive)
   */
  function getProviderByName(name: string): ProviderWithStatus | undefined {
    const lowerName = name.toLowerCase();
    return providers.value.find((p) => p.name.toLowerCase() === lowerName);
  }

  /**
   * Load all providers from backend
   */
  async function loadProviders(): Promise<void> {
    isLoading.value = true;
    error.value = null;

    try {
      providers.value = await invoke<ProviderWithStatus[]>('get_providers');
    } catch (e) {
      error.value = '加载 Provider 列表失败';
      console.error('Failed to load providers:', e);
      throw e;
    } finally {
      isLoading.value = false;
    }
  }

  /**
   * Save a provider (create or update)
   */
  async function saveProvider(request: SaveProviderRequest): Promise<void> {
    isSaving.value = true;
    error.value = null;

    try {
      await invoke('save_provider', { request });
      // Reload providers to get updated list
      await loadProviders();
    } catch (e) {
      error.value = '保存 Provider 失败';
      console.error('Failed to save provider:', e);
      throw e;
    } finally {
      isSaving.value = false;
    }
  }

  /**
   * Delete a provider by ID
   */
  async function deleteProvider(id: string): Promise<boolean> {
    isSaving.value = true;
    error.value = null;

    try {
      const deleted = await invoke<boolean>('delete_provider', { id });
      if (deleted) {
        await loadProviders();
      }
      return deleted;
    } catch (e) {
      error.value = '删除 Provider 失败';
      console.error('Failed to delete provider:', e);
      throw e;
    } finally {
      isSaving.value = false;
    }
  }

  /**
   * Test connection to a provider
   */
  async function testConnection(id: string): Promise<TestConnectionResult> {
    testingProviderId.value = id;
    testResult.value = null;
    error.value = null;

    try {
      const result = await invoke<TestConnectionResult>('test_provider_connection', { id });
      testResult.value = result;
      return result;
    } catch (e) {
      error.value = '测试连接失败';
      console.error('Failed to test connection:', e);
      throw e;
    } finally {
      testingProviderId.value = null;
    }
  }

  /**
   * Check if a provider is currently being tested
   */
  function isTestingProvider(id: string): boolean {
    return testingProviderId.value === id;
  }

  /**
   * Toggle provider enabled status
   */
  async function toggleProvider(id: string): Promise<void> {
    const provider = getProvider(id);
    if (!provider) {
      throw new Error(`Provider not found: ${id}`);
    }

    await saveProvider({
      ...provider,
      enabled: !provider.enabled,
    });
  }

  /**
   * Get models for a provider
   */
  function getModels(providerId: string): string[] {
    const provider = getProvider(providerId);
    return provider?.available_models ?? [];
  }

  /**
   * Get default model for a provider
   */
  function getDefaultModel(providerId: string): string | undefined {
    const provider = getProvider(providerId);
    return provider?.default_model;
  }

  /**
   * Check if a provider has an API key stored
   */
  function hasApiKey(providerId: string): boolean {
    const provider = getProvider(providerId);
    return provider?.has_api_key ?? false;
  }

  /**
   * Clear error state
   */
  function clearError(): void {
    error.value = null;
  }

  /**
   * Reset all state
   */
  function reset(): void {
    providers.value = [];
    isLoading.value = false;
    isSaving.value = false;
    error.value = null;
    testingProviderId.value = null;
    testResult.value = null;
  }

  return {
    // State (readonly to prevent external mutation)
    providers: readonly(providers),
    isLoading: readonly(isLoading),
    isSaving: readonly(isSaving),
    error: readonly(error),
    testingProviderId: readonly(testingProviderId),
    testResult: readonly(testResult),

    // Computed
    enabledProviders,
    providerCount,
    hasProviders,

    // Actions
    loadProviders,
    saveProvider,
    deleteProvider,
    testConnection,
    toggleProvider,

    // Helpers
    getProvider,
    getProviderByName,
    getModels,
    getDefaultModel,
    hasApiKey,
    isTestingProvider,
    clearError,
    reset,
  };
}

/**
 * Singleton instance for global provider state
 * Use this when you need to share provider state across components
 */
let globalInstance: ReturnType<typeof useProviders> | null = null;

export function useProvidersGlobal(): ReturnType<typeof useProviders> {
  if (!globalInstance) {
    globalInstance = useProviders();
  }
  return globalInstance;
}
