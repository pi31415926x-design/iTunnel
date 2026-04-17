<script setup lang="ts">
import {
    Bars4Icon,
    //Cog6ToothIcon,
    CursorArrowRippleIcon,
    CreditCardIcon
} from "@heroicons/vue/24/outline";
//import { APP_CONFIG } from "../config/app";


defineProps<{
    collapsed: boolean;
    open: boolean;
}>();

defineEmits(["toggle", "close"]);

const items = [
    { name: "Overview", icon: CursorArrowRippleIcon, to: "/" },
    // { name: "Settings", icon: Cog6ToothIcon, to: "/settings" },
    { name: "Subscription", icon: CreditCardIcon, to: "/subscribe" },
    { name: "Logs", icon: Bars4Icon, to: "/logs" }
];
</script>

<template>
    <aside :class="[
        'fixed md:static',
        'left-0 top-0 md:top-0 bottom-0',
        'z-30',
        'flex flex-col',
        'bg-white dark:bg-slate-950',
        'border-r border-slate-200 dark:border-slate-800',
        'transition-all duration-200',
        collapsed ? 'w-16' : 'w-auto min-w-fit',
        // Mobile: translate based on open state
        open ? 'translate-x-0' : '-translate-x-full md:translate-x-0'
    ]">
        <nav class="flex-1 overflow-y-auto p-3 md:pt-3" :class="collapsed ? '' : 'pr-[calc(0.75rem+5px)]'">
            <!-- Header  
            <div class="flex items-center gap-3 px-3 py-2 text-lg text-slate-700 dark:text-slate-300">
                <component :is="RocketLaunchIcon" class="h-4 w-4 shrink-0" />
                <span v-show="!collapsed" class="truncate whitespace-nowrap overflow-hidden">
                    {{ APP_CONFIG.shortName }}
                </span>
            </div>
            -->
            <!-- Menu -->
            <router-link v-for="item in items" :key="item.name" :to="item.to" @click="$emit('close')" class="
                    flex items-center gap-3
                    rounded px-3 py-2
                    text-sm text-slate-600
                    hover:bg-slate-50 hover:text-slate-900
                    dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100
                " active-class="bg-slate-50 text-slate-900 font-medium dark:bg-slate-800">
                <component :is="item.icon" class="h-4 w-4 shrink-0" />
                <span v-show="!collapsed" class="truncate whitespace-nowrap overflow-hidden">
                    {{ item.name }}
                </span>
            </router-link>
        </nav>
    </aside>
</template>
