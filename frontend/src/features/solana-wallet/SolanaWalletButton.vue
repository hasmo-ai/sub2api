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
          @click="disconnect"
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
            @click="connect(wallet)"
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
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import { useI18n } from 'vue-i18n'
import { Coins } from '@lucide/vue'
import { getWallets } from '@wallet-standard/app'
import type { Wallet, WalletAccount } from '@wallet-standard/base'
import {
  StandardConnect,
  StandardDisconnect,
  StandardEvents,
  type StandardConnectFeature,
  type StandardDisconnectFeature,
  type StandardEventsFeature,
} from '@wallet-standard/features'
import {
  SolanaSignMessage,
  type SolanaSignMessageFeature,
} from '@solana/wallet-standard-features'
import BaseDialog from '@/components/common/BaseDialog.vue'
import Icon from '@/components/icons/Icon.vue'

type SolanaWallet = Omit<Wallet, 'features'> & {
  readonly features: Wallet['features'] & StandardConnectFeature & SolanaSignMessageFeature
}

const { t } = useI18n()
const walletRegistry = getWallets()
const wallets = shallowRef<SolanaWallet[]>([])
const selectedWallet = shallowRef<SolanaWallet | null>(null)
const account = shallowRef<WalletAccount | null>(null)
const dialogOpen = ref(false)
const connectingWalletName = ref('')
const errorMessage = ref('')

let removeWalletChangeListener: (() => void) | undefined
let removeRegisterListener: (() => void) | undefined
let removeUnregisterListener: (() => void) | undefined

const isConnecting = computed(() => Boolean(connectingWalletName.value))
const shortenedAddress = computed(() => {
  if (!account.value) return ''
  return `${account.value.address.slice(0, 4)}...${account.value.address.slice(-4)}`
})

function isSolanaAccount(candidate: WalletAccount): boolean {
  return candidate.chains.some((chain) => chain.startsWith('solana:'))
}

function isSolanaWallet(candidate: Wallet): candidate is SolanaWallet {
  return candidate.chains.some((chain) => chain.startsWith('solana:'))
    && StandardConnect in candidate.features
    && SolanaSignMessage in candidate.features
}

function refreshWallets(): void {
  wallets.value = walletRegistry.get().filter(isSolanaWallet)
  if (selectedWallet.value && !wallets.value.includes(selectedWallet.value)) {
    clearConnection()
  }
}

function watchWalletAccounts(wallet: SolanaWallet): void {
  removeWalletChangeListener?.()
  const events = wallet.features[StandardEvents] as StandardEventsFeature[typeof StandardEvents] | undefined
  removeWalletChangeListener = events?.on('change', ({ accounts }) => {
    if (!accounts) return
    account.value = accounts.find(isSolanaAccount) ?? null
    if (!account.value) clearConnection()
  })
}

async function connect(wallet: SolanaWallet): Promise<void> {
  connectingWalletName.value = wallet.name
  errorMessage.value = ''

  try {
    const result = await wallet.features[StandardConnect].connect()
    const solanaAccount = result.accounts.find(isSolanaAccount)
    if (!solanaAccount) throw new Error('Wallet returned no Solana account')

    selectedWallet.value = wallet
    account.value = solanaAccount
    watchWalletAccounts(wallet)
    dialogOpen.value = false
  } catch (error) {
    console.warn('Solana wallet connection failed:', error)
    errorMessage.value = t('solanaWallet.connectFailed')
  } finally {
    connectingWalletName.value = ''
  }
}

async function disconnect(): Promise<void> {
  const wallet = selectedWallet.value
  errorMessage.value = ''

  try {
    const disconnectFeature = wallet?.features[StandardDisconnect] as StandardDisconnectFeature[typeof StandardDisconnect] | undefined
    await disconnectFeature?.disconnect()
  } catch (error) {
    console.warn('Solana wallet disconnect failed:', error)
  } finally {
    clearConnection()
    dialogOpen.value = false
  }
}

function clearConnection(): void {
  removeWalletChangeListener?.()
  removeWalletChangeListener = undefined
  selectedWallet.value = null
  account.value = null
}

function closeDialog(): void {
  if (!isConnecting.value) dialogOpen.value = false
}

onMounted(() => {
  refreshWallets()
  removeRegisterListener = walletRegistry.on('register', refreshWallets)
  removeUnregisterListener = walletRegistry.on('unregister', refreshWallets)
})

onBeforeUnmount(() => {
  removeRegisterListener?.()
  removeUnregisterListener?.()
  removeWalletChangeListener?.()
})
</script>
