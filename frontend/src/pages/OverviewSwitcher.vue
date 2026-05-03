<template>
  <component :is="activeComponent" />
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent } from 'vue';
import { useWireGuardStore } from '@/stores/wireguard';

/** Async so the inactive overview is not loaded at runtime (bundler may still list both chunks). */
const ClientOverview = defineAsyncComponent(() => import('./ClientOverview.vue'));
const ServerOverview = defineAsyncComponent(() => import('./ServerOverview.vue'));

const wireguardStore = useWireGuardStore();

const activeComponent = computed(() => {
  return wireguardStore.mode === 'server' ? ServerOverview : ClientOverview;
});
</script>
