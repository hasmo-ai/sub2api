<template>
  <div>
    <button
      type="button"
      class="flex h-9 items-center gap-2 rounded-lg px-2.5 text-sm font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500/30 disabled:cursor-wait disabled:opacity-60"
      :class="account
        ? 'bg-emerald-50 text-emerald-700 hover:bg-emerald-100 dark:bg-emerald-900/20 dark:text-emerald-300 dark:hover:bg-emerald-900/30'
        : 'text-gray-600 hover:bg-gray-100 hover:text-gray-900 dark:text-dark-400 dark:hover:bg-dark-800 dark:hover:text-white'"
      :aria-label="account ? t('solanaWallet.connectedAs', { address: account.address }) : t('solanaWallet.connect')"
      :title="account ? account.address : t('solanaWallet.connect')"
      :disabled="isConnecting"
      data-testid="solana-wallet-trigger"
      @click="dialogOpen = true"
    >
      <Coins class="h-4 w-4" :stroke-width="1.5" />
      <span class="hidden xl:inline">
        {{ account ? shortenedAddress : t('solanaWallet.connect') }}
      </span>
      <span
        v-if="account"
        class="h-2 w-2 rounded-full bg-emerald-500"
        aria-hidden="true"
      ></span>
    </button>

    <BaseDialog
      :show="dialogOpen"
      :title="account ? t('solanaWallet.wallet') : t('solanaWallet.selectWallet')"
      width="narrow"
      @close="closeDialog"
    >
      <div v-if="account && selectedWallet" class="space-y-5">
        <div class="flex items-center gap-3">
          <img
            :src="selectedWallet.icon"
            :alt="selectedWallet.name"
            class="h-10 w-10 rounded-lg"
          >
          <div class="min-w-0">
            <p class="font-medium text-gray-900 dark:text-white">{{ selectedWallet.name }}</p>
            <p class="truncate font-mono text-xs text-gray-500 dark:text-dark-400">
              {{ account.address }}
            </p>
          </div>
        </div>

        <button
          type="button"
          class="btn btn-secondary w-full text-red-600 dark:text-red-400"
          data-testid="solana-wallet-disconnect"
          @click="disconnectWallet"
        >
          {{ t('solanaWallet.disconnect') }}
        </button>
      </div>

      <div v-else>
        <div v-if="wallets.length" class="space-y-2">
          <button
            v-for="wallet in wallets"
            :key="wallet.name"
            type="button"
            class="flex w-full items-center gap-3 rounded-lg border border-gray-200 px-3 py-2.5 text-left transition-colors hover:border-primary-300 hover:bg-primary-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500/30 dark:border-dark-700 dark:hover:border-primary-700 dark:hover:bg-primary-900/20"
            :disabled="isConnecting"
            :data-wallet-name="wallet.name"
            @click="connectWallet(wallet)"
          >
            <img :src="wallet.icon" :alt="wallet.name" class="h-9 w-9 rounded-lg">
            <span class="min-w-0 flex-1 truncate font-medium text-gray-900 dark:text-white">
              {{ wallet.name }}
            </span>
            <span
              v-if="connectingWalletName === wallet.name"
              class="text-xs text-gray-500 dark:text-dark-400"
            >
              {{ t('solanaWallet.connecting') }}
            </span>
            <Icon v-else name="chevronRight" size="sm" class="text-gray-400" />
          </button>
        </div>

        <div v-else class="py-4 text-center">
          <Icon name="wallet" size="xl" class="mx-auto text-gray-400" />
          <p class="mt-3 font-medium text-gray-900 dark:text-white">
            {{ t('solanaWallet.noWallet') }}
          </p>
          <p class="mt-1 text-sm text-gray-500 dark:text-dark-400">
            {{ t('solanaWallet.noWalletDescription') }}
          </p>
        </div>

        <p
          v-if="errorMessage"
          class="mt-3 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300"
          role="alert"
        >
          {{ errorMessage }}
        </p>
      </div>
    </BaseDialog>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Coins } from '@lucide/vue'
import BaseDialog from '@/components/common/BaseDialog.vue'
import Icon from '@/components/icons/Icon.vue'
import { useSolanaWallet, type SolanaWallet } from './useSolanaWallet'

const { t } = useI18n()
const {
  wallets,
  selectedWallet,
  account,
  connectingWalletName,
  isConnecting,
  shortenedAddress,
  connect,
  disconnect,
  initSolanaWallet,
  teardownSolanaWallet,
} = useSolanaWallet()

const dialogOpen = ref(false)
const errorMessage = ref('')

async function connectWallet(wallet: SolanaWallet): Promise<void> {
  errorMessage.value = ''
  const connected = await connect(wallet)
  if (connected) {
    dialogOpen.value = false
  } else {
    errorMessage.value = t('solanaWallet.connectFailed')
  }
}

async function disconnectWallet(): Promise<void> {
  errorMessage.value = ''
  await disconnect()
  dialogOpen.value = false
}

function closeDialog(): void {
  if (!isConnecting.value) dialogOpen.value = false
}

onMounted(() => {
  initSolanaWallet()
})

onBeforeUnmount(() => {
  teardownSolanaWallet()
})
</script>
