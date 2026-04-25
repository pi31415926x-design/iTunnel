<template>
  <component :is="activeComponent" />
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useWireGuardStore } from '@/stores/wireguard';
import ClientOverview from './ClientOverview.vue';
import ServerOverview from './ServerOverview.vue';

const wireguardStore = useWireGuardStore();

const activeComponent = computed(() => {
  return wireguardStore.mode === 'server' ? ServerOverview : ClientOverview;
});
</script>
