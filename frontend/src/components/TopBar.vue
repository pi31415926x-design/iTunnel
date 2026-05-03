<script setup lang="ts">
import { Bars3Icon } from "@heroicons/vue/24/outline";
import { useTheme } from "./useTheme";
import {
    MoonIcon,
    SunIcon
} from "@heroicons/vue/24/outline";
import { APP_CONFIG } from "../config/app";
import { useWireGuardStore } from "@/stores/wireguard";
import { serverFetch } from "@/server-fetch";

defineProps<{
    sidebarOpen: boolean;
}>();

defineEmits(["toggleSidebar"]);
const { isDark, toggleTheme } = useTheme();
const wireguardStore = useWireGuardStore();

async function serverLogout() {
  try {
    await serverFetch("/api/logout", { method: "POST" });
  } catch {
    /* ignore */
  }
  window.location.assign("/login");
}
</script>

<template>
  <header
    :class="[
      'h-14 flex items-center justify-between',
      'border-b border-slate-200 dark:border-slate-800',
      'bg-white dark:bg-slate-950',
      'px-4',
      'relative z-40',
      'transition-all duration-200',
      // Mobile: shift right when sidebar is open
      sidebarOpen ? 'md:ml-0 ml-16' : 'ml-0'
    ]"
  >
    <!-- Left -->
    <button
      @click="$emit('toggleSidebar')"
      class="
        rounded p-2
        text-slate-500
        hover:bg-slate-100 dark:hover:bg-slate-800
      "
    >
      <Bars3Icon class="h-5 w-5" />
    </button>
    <span class="text-lg text-center text-gray-500 dark:text-gray-400 italic truncate mx-4 flex-1">{{ APP_CONFIG.longName }}</span>
    <!-- Right -->
    <button
      v-if="wireguardStore.mode === 'server'"
      type="button"
      @click="serverLogout"
      class="mr-1 rounded px-2 py-1 text-xs font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800"
    >
      Log out
    </button>
    <button
      @click="toggleTheme"
      class="
        rounded p-2
        text-slate-500
        hover:bg-slate-100 dark:hover:bg-slate-800
      "
    >
      <span v-if="isDark"><component :is="MoonIcon" class="h-4 w-4 shrink-0" /></span>
      <span v-else><component :is="SunIcon" class="h-4 w-4 shrink-0" /></span>
    </button>
  </header>
</template>
