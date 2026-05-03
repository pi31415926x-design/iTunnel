<template>
  <div
    class="min-h-screen flex items-center justify-center p-6 bg-slate-50/50 dark:bg-slate-950/50 text-slate-900 dark:text-slate-100">
    <div class="w-full max-w-md">
      <div
        class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl shadow-xl shadow-slate-200/50 dark:shadow-none overflow-hidden">
        <div class="px-8 pt-8 pb-2 text-center">
          <div
            class="mx-auto w-12 h-12 rounded-xl dark:bg-red-500/10 text-red-600 dark:text-red-400 flex items-center justify-center mb-4">
            <ShieldCheckIcon class="h-7 w-7" />
          </div>
          <h1 class="text-xl font-bold tracking-tight">Server sign in</h1>
          <p class="mt-1 text-sm text-slate-500 dark:text-slate-400">
            Enter the operator credentials from your <span class="font-mono text-xs">.env</span> file.
          </p>
        </div>

        <form class="px-8 pb-8 space-y-4" @submit.prevent="submit">
          <div>
            <label class="block text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-1"
              for="login-user">Username</label>
            <input id="login-user" v-model.trim="username" type="text" autocomplete="username" required
              class="w-full rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-red-500/40" />
          </div>
          <div>
            <label class="block text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-1"
              for="login-pwd">Password</label>
            <input id="login-pwd" v-model="password" type="password" autocomplete="current-password" required
              class="w-full rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-red-500/40" />
          </div>

          <p v-if="errorMsg" class="text-sm text-red-600 dark:text-red-400">{{ errorMsg }}</p>

          <button type="submit" :disabled="submitting"
            class="w-full inline-flex justify-center items-center gap-2 rounded-lg bg-red-600 hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed text-white text-sm font-semibold py-2.5 shadow-sm shadow-red-500/20 transition-colors">
            <span v-if="submitting" class="inline-block h-4 w-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
            <span>{{ submitting ? 'Signing in…' : 'Sign in' }}</span>
          </button>
        </form>
      </div>
      <p class="mt-6 text-center text-xs text-slate-400 dark:text-slate-500">
        Session lasts 15 minutes, then you must sign in again.
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { ShieldCheckIcon } from '@heroicons/vue/24/solid';
import { serverFetch } from '@/server-fetch';

const router = useRouter();
const username = ref('');
const password = ref('');
const errorMsg = ref('');
const submitting = ref(false);

onMounted(async () => {
  try {
    const res = await fetch('/api/auth/status', { credentials: 'include' });
    const j = await res.json();
    if (j.authenticated) {
      await router.replace('/');
    }
  } catch {
    /* ignore */
  }
});

async function submit() {
  if (submitting.value) return;
  errorMsg.value = '';
  submitting.value = true;
  try {
    const res = await serverFetch('/api/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        username: username.value,
        password: password.value,
      }),
    });
    const j = await res.json().catch(() => ({}));
    if (res.ok && j.success) {
      await router.replace('/');
      return;
    }
    errorMsg.value = j.message || 'Sign in failed';
  } catch {
    errorMsg.value = 'Could not reach the server';
  } finally {
    submitting.value = false;
  }
}
</script>
