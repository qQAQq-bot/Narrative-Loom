<script setup lang="ts">
import { useTheme } from '@/composables/useTheme';
import TitleBar from '@/components/common/TitleBar.vue';
import ResizeGrip from '@/components/common/ResizeGrip.vue';

// Initialize theme system
useTheme();
</script>

<template>
  <div class="h-screen flex flex-col overflow-hidden bg-fabric-cream">
    <TitleBar />
    <div class="flex-1 overflow-hidden">
      <router-view />
    </div>
    <ResizeGrip />
  </div>
</template>
