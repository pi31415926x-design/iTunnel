<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import Sidebar from "../components/Sidebar.vue";
import Topbar from "../components/TopBar.vue";
import { APP_CONFIG } from "../config/app.ts";

const sidebarCollapsed = ref(false);
const sidebarOpen = ref(false); // mobile drawer

// Handle toggle based on screen size
const handleToggleSidebar = () => {
    const isMobile = window.innerWidth < 768; // md breakpoint
    if (isMobile) {
        sidebarOpen.value = !sidebarOpen.value;
    } else {
        sidebarCollapsed.value = !sidebarCollapsed.value;
    }
};

// Initialize collapsed state on mobile
const updateSidebarState = () => {
    const isMobile = window.innerWidth < 768;
    if (isMobile) {
        sidebarCollapsed.value = true; // Always collapsed (icon-only) on mobile
        sidebarOpen.value = false; // Closed by default
    }
};

onMounted(() => {
    updateSidebarState();
    window.addEventListener('resize', updateSidebarState);
});

onUnmounted(() => {
    window.removeEventListener('resize', updateSidebarState);
});
</script>

<template>
    <div class="
      min-h-screen flex
      bg-slate-50 text-slate-900
      dark:bg-slate-900 dark:text-slate-100
    ">
        <!-- Mobile overlay -->
        <div v-if="sidebarOpen" class="fixed inset-0 z-20 bg-black/30 md:hidden" @click="sidebarOpen = false" />
        
        <!-- Sidebar -->
        <Sidebar 
            :collapsed="sidebarCollapsed" 
            :open="sidebarOpen" 
            @toggle="sidebarCollapsed = !sidebarCollapsed"
            @close="sidebarOpen = false" 
        />
        
        <!-- Main -->
        <div class="flex flex-1 flex-col w-full md:w-auto min-h-screen">
            <!-- Topbar -->
            <Topbar 
                :sidebar-open="sidebarOpen"
                @toggleSidebar="handleToggleSidebar" 
            />
            
            <!-- Content -->
            <main class="flex-1 w-full">
                <router-view />
            </main>
            
            <!-- Footer -->
            <footer class="
                mt-auto
                pt-8
                text-center text-xs
                text-gray-400
                dark:text-gray-400
            ">
                {{ APP_CONFIG.copyright }}
            </footer>
        </div>
    </div>
</template>
