<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { CheckIcon, SparklesIcon } from "@heroicons/vue/20/solid";

const deviceId = ref('');
const loading = ref(false);
const plansLoading = ref(true);
const message = ref({ text: '', type: '' });
const showPaymentModal = ref(false);
const qrCodeData = ref('');
const selectedPlan = ref<any>(null);
const device_type = ref('desktop');
const plans = ref<any[]>([]);

onMounted(async () => {
  try {
    const res = await fetch('/api/subscribe_plans');
    if (res.ok) {
      plans.value = await res.json();
    }
  } catch (err) {
    console.error('Failed to fetch plans:', err);
  } finally {
    plansLoading.value = false;
  }
});

const handleSubscribe = async (plan: any) => {
  if (loading.value) return;
  loading.value = true;
  message.value = { text: '', type: '' };
  selectedPlan.value = plan;

  try {
    const res = await fetch('/api/subscribe_req', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        device_id: deviceId.value,
        subscribe_type: plan.name,
        price: parseFloat(plan.price),
        device_type: device_type.value
      })
    });

    if (res.ok) {
      const data = await res.json();
      qrCodeData.value = data.image_base64;
      showPaymentModal.value = true;
      message.value = { text: 'Subscription request sent successfully!', type: 'success' };
    } else {
      const errText = await res.text();
      message.value = { text: `Failed to send request: ${errText}`, type: 'error' };
    }
  } catch (err) {
    message.value = { text: `Connection error: ${err}`, type: 'error' };
  } finally {
    loading.value = false;
  }
};
</script>

<template>
  <div class="h-full overflow-y-auto bg-slate-50 dark:bg-slate-950 p-6 md:p-10">
    <div class="mx-auto max-w-7xl">
      <!-- Header -->
      <div class="text-center mb-12">
        <h2 class="text-3xl font-bold tracking-tight text-slate-900 dark:text-white sm:text-4xl">
          Choose Your Plan
        </h2>
        <p class="mt-4 text-lg leading-8 text-slate-600 dark:text-slate-400">
          Unlock high-speed, secure browsing with our premium service.
        </p>

        <!-- Feedback Message -->
        <div v-if="message.text" :class="[
          'mt-6 inline-block px-4 py-2 rounded-lg text-sm font-medium',
          message.type === 'success' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
        ]">
          {{ message.text }}
        </div>
      </div>

      <!-- Pricing Grid -->
      <div v-if="plansLoading" class="grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-4">
        <div v-for="i in 4" :key="i"
          class="h-96 rounded-2xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 animate-pulse p-8 flex flex-col gap-6">
          <div class="h-6 w-1/2 bg-slate-100 dark:bg-slate-800 rounded"></div>
          <div class="h-10 w-3/4 bg-slate-100 dark:bg-slate-800 rounded"></div>
          <div class="flex-1 space-y-4">
            <div v-for="j in 4" :key="j" class="h-4 w-full bg-slate-50 dark:bg-slate-800/50 rounded"></div>
          </div>
          <div class="h-12 w-full bg-slate-100 dark:bg-slate-800 rounded"></div>
        </div>
      </div>

      <div v-else class="grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-4">
        <div v-for="plan in plans" :key="plan.name" :class="[
          'relative flex flex-col p-8 rounded-2xl transition-all duration-300',
          plan.popular
            ? 'bg-blue-600 ring-2 ring-blue-500 shadow-xl scale-105 z-10'
            : 'bg-white dark:bg-slate-900 ring-1 ring-slate-200 dark:ring-slate-800 hover:ring-blue-300 dark:hover:ring-blue-800 hover:shadow-lg',
        ]">
          <!-- Popular Badge -->
          <div v-if="plan.popular"
            class="absolute -top-4 left-1/2 -translate-x-1/2 bg-white text-blue-600 text-xs font-bold px-3 py-1 rounded-full shadow-sm flex items-center gap-1">
            <SparklesIcon class="h-3 w-3" />
            MOST POPULAR
          </div>

          <div class="mb-8">
            <h3 :class="[
              'text-lg font-semibold leading-8',
              plan.popular ? 'text-white' : 'text-slate-900 dark:text-slate-100',
            ]">
              {{ plan.name }}
            </h3>
            <div class="mt-4 flex items-baseline gap-x-1">
              <span :class="[
                'text-4xl font-bold tracking-tight',
                plan.popular ? 'text-white' : 'text-slate-900 dark:text-slate-100',
              ]">
                ¥{{ plan.price }}
              </span>
              <span :class="[
                'text-sm font-semibold leading-6',
                plan.popular ? 'text-blue-100' : 'text-slate-600 dark:text-slate-400',
              ]">
                / {{ plan.duration }}
              </span>
            </div>
          </div>

          <ul role="list" class="space-y-3 text-sm leading-6 flex-1 mb-8"
            :class="plan.popular ? 'text-blue-50' : 'text-slate-600 dark:text-slate-400'">
            <li v-for="feature in plan.features" :key="feature" class="flex gap-x-3">
              <CheckIcon :class="[
                'h-6 w-5 flex-none',
                plan.popular ? 'text-white' : 'text-blue-600 dark:text-blue-400',
              ]" aria-hidden="true" />
              {{ feature }}
            </li>
          </ul>

          <button @click="handleSubscribe(plan)" :disabled="loading" :class="[
            'w-full py-2.5 px-4 rounded-xl text-center text-sm font-semibold transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed',
            plan.popular
              ? 'bg-white text-blue-600 hover:bg-slate-50'
              : 'bg-slate-900 text-white hover:bg-slate-800 dark:bg-blue-600 dark:hover:bg-blue-500',
          ]">
            <span v-if="loading && message.type === ''">Processing...</span>
            <span v-else>{{ plan.cta }}</span>
          </button>
        </div>
      </div>

      <!-- FAQ or Footer -->
      <div class="mt-16 text-center">
        <p class="text-sm text-slate-500 dark:text-slate-500">
          All plans include a 30-day money-back guarantee. Secure payment processing.
        </p>
      </div>
    </div>

    <!-- WeChat Payment Modal -->
    <Transition name="fade">
      <div v-if="showPaymentModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-sm">
        <div
          class="bg-white dark:bg-slate-900 w-full max-w-sm rounded-3xl shadow-2xl overflow-hidden animate-in fade-in zoom-in duration-300">
          <!-- Modal Header -->
          <div
            class="px-6 py-6 border-b border-slate-100 dark:border-slate-800 flex items-center justify-between bg-slate-50/50 dark:bg-slate-800/50">
            <div class="flex items-center gap-3">
              <h3 class="text-lg font-bold text-slate-900 dark:text-white">WeChat Pay</h3>
            </div>
            <button @click="showPaymentModal = false"
              class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors">
              <svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
          <!-- Modal Body -->
          <div class="p-8 text-center">
            <div class="mb-6">
              <div v-if="selectedPlan"
                class="text-slate-600 dark:text-slate-400 text-sm mb-1 uppercase tracking-wider font-semibold">
                {{ selectedPlan.name }} Plan
              </div>
              <div v-if="selectedPlan" class="text-4xl font-extrabold text-slate-900 dark:text-white">
                ¥{{ selectedPlan.price }}
              </div>
            </div>

            <!-- QR Code Container -->
            <div
              class="relative mx-auto w-56 h-56 p-4 bg-slate-50 dark:bg-slate-800 rounded-2xl border border-slate-100 dark:border-slate-700 shadow-inner group">
              <img :src="qrCodeData" alt="Payment QR Code"
                class="w-full h-full object-contain rounded-lg transition-transform group-hover:scale-105 duration-300" />
              <div v-if="!qrCodeData" class="absolute inset-0 flex items-center justify-center">
                <div class="w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
              </div>
            </div>

            <p class="mt-8 text-sm text-slate-500 dark:text-slate-400 leading-relaxed">
              Open <strong>WeChat</strong> and scan the QR code to complete your purchase safely.
            </p>
          </div>

          <!-- Modal Footer -->
          <div class="px-8 py-6 bg-slate-50 dark:bg-slate-800/50 flex flex-col gap-3">
            <button @click="showPaymentModal = false"
              class="w-full py-3 bg-slate-900 dark:bg-blue-600 text-white rounded-xl font-bold hover:opacity-90 transition-opacity">
              I've paid
            </button>
            <p class="text-[10px] text-center text-slate-400 dark:text-slate-500">
              The subscription will be activated automatically after payment.
            </p>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.glass-effect {
  backdrop-filter: blur(16px) saturate(180%);
  -webkit-backdrop-filter: blur(16px) saturate(180%);
}
</style>
