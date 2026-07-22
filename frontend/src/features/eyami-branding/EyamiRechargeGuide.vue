<template>
  <section
    class="card overflow-hidden border-gray-200 dark:border-dark-700"
    data-testid="eyami-recharge-guide"
  >
    <div class="border-b border-gray-100 px-6 py-5 dark:border-dark-700">
      <div class="flex items-start gap-3">
        <div
          class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-emerald-100 dark:bg-emerald-900/30"
        >
          <Icon name="creditCard" size="md" class="text-emerald-600 dark:text-emerald-400" />
        </div>
        <div>
          <h2 class="text-base font-semibold text-gray-900 dark:text-white">
            {{ t('redeem.rechargeGuideTitle') }}
          </h2>
          <p class="mt-1 text-sm leading-6 text-gray-600 dark:text-dark-300">
            {{ t('redeem.rechargeGuideLead') }}
          </p>
        </div>
      </div>
    </div>

    <div class="p-6">
      <p class="mb-4 text-sm font-medium text-emerald-700 dark:text-emerald-400">
        {{ t('redeem.rechargeRate') }}
      </p>
      <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <a
          v-for="option in rechargeOptions"
          :key="option.href"
          :href="option.href"
          target="_blank"
          rel="noopener noreferrer"
          class="btn btn-secondary min-w-0 flex-col gap-1 px-3 py-3 text-center"
          :aria-label="t('redeem.rechargeOptionLabel', { price: option.price, credit: option.credit })"
        >
          <span class="text-base font-semibold text-gray-900 dark:text-white">{{ option.price }}</span>
          <span class="inline-flex items-center gap-1 text-xs text-gray-500 dark:text-dark-400">
            {{ option.credit }} API
            <Icon name="externalLink" size="xs" />
          </span>
        </a>
      </div>
      <p class="mt-4 text-xs leading-5 text-gray-500 dark:text-dark-400">
        {{ t('redeem.rechargeNote') }}
      </p>
    </div>
  </section>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import Icon from '@/components/icons/Icon.vue'

const { t } = useI18n()

const rechargeOptions = [
  { price: '¥10', credit: '$20', href: 'https://pay.ldxp.cn/item/yrhxzl' },
  { price: '¥25', credit: '$50', href: 'https://pay.ldxp.cn/item/v3gmy3' },
  { price: '¥50', credit: '$100', href: 'https://pay.ldxp.cn/item/ssoboq' },
  { price: '¥100', credit: '$200', href: 'https://pay.ldxp.cn/item/dm6ylc' }
] as const
</script>
