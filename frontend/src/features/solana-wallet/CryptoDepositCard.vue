<template>
  <div v-if="config" class="card p-6" data-testid="crypto-deposit-card">
    <div class="flex items-center justify-between">
      <h3 class="text-base font-semibold text-gray-900 dark:text-white">
        {{ t('cryptoDeposit.title') }}
      </h3>
      <span
        v-if="config.cluster === 'devnet'"
        class="rounded-md border border-amber-200 bg-amber-50 px-2 py-0.5 text-xs font-medium text-amber-700 dark:border-amber-800 dark:bg-amber-900/20 dark:text-amber-300"
      >
        Devnet
      </span>
    </div>

    <template v-if="!txSignature">
      <!-- Currency selector -->
      <div class="mt-4 flex space-x-1 rounded-xl bg-gray-100 p-1 dark:bg-dark-800">
        <button
          v-for="option in currencyOptions"
          :key="option"
          type="button"
          class="flex-1 rounded-lg px-4 py-2 text-sm font-medium transition-all"
          :class="currency === option
            ? 'bg-white text-gray-900 shadow dark:bg-dark-700 dark:text-white'
            : 'text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'"
          :data-currency="option"
          @click="currency = option"
        >
          {{ option }}
        </button>
      </div>

      <!-- Amount input -->
      <div class="mt-4">
        <label class="text-xs font-medium text-gray-400 dark:text-gray-500" for="crypto-deposit-amount">
          {{ t('cryptoDeposit.amount') }}
        </label>
        <div class="relative mt-1">
          <input
            id="crypto-deposit-amount"
            v-model="amount"
            type="number"
            min="0"
            step="any"
            :placeholder="t('cryptoDeposit.amountPlaceholder')"
            class="input w-full pr-16"
            data-testid="crypto-deposit-amount"
          >
          <span class="pointer-events-none absolute inset-y-0 right-3 flex items-center text-sm text-gray-400">
            {{ currency }}
          </span>
        </div>
        <p class="mt-2 text-xs text-gray-500 dark:text-gray-400">
          {{ t('cryptoDeposit.minHint', { usd: config.min_deposit_usd }) }}
        </p>
      </div>

      <!-- Wallet hint / submit -->
      <p
        v-if="!account"
        class="mt-4 rounded-lg bg-gray-50 px-3 py-2 text-sm text-gray-500 dark:bg-dark-800 dark:text-dark-400"
        data-testid="crypto-deposit-connect-hint"
      >
        {{ t('cryptoDeposit.connectHint') }}
      </p>
      <button
        v-else
        type="button"
        class="btn btn-primary mt-4 w-full py-3 text-base font-medium"
        :disabled="!canSubmit || submitting"
        data-testid="crypto-deposit-submit"
        @click="handleSubmit"
      >
        <span v-if="submitting" class="flex items-center justify-center gap-2">
          <span class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"></span>
          {{ t('common.processing') }}
        </span>
        <span v-else>{{ t('cryptoDeposit.submit') }}</span>
      </button>

      <p
        v-if="errorMessage"
        class="mt-3 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300"
        role="alert"
        data-testid="crypto-deposit-error"
      >
        {{ errorMessage }}
      </p>
    </template>

    <!-- Success state -->
    <div v-else class="mt-4 space-y-3" data-testid="crypto-deposit-success">
      <p class="font-medium text-emerald-600 dark:text-emerald-400">
        {{ t('cryptoDeposit.successTitle') }}
      </p>
      <p class="break-all font-mono text-xs text-gray-500 dark:text-dark-400">{{ txSignature }}</p>
      <a
        :href="explorerUrl"
        target="_blank"
        rel="noopener noreferrer"
        class="inline-block text-sm font-medium text-primary-600 hover:underline dark:text-primary-400"
      >
        {{ t('cryptoDeposit.viewOnExplorer') }}
      </a>
      <p class="text-sm text-gray-500 dark:text-gray-400">
        {{ t('cryptoDeposit.successNote') }}
      </p>
      <button
        type="button"
        class="btn btn-secondary w-full"
        data-testid="crypto-deposit-again"
        @click="resetForm"
      >
        {{ t('cryptoDeposit.again') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { AnchorProvider, BN, Program, type Idl, type Wallet as AnchorWallet } from '@coral-xyz/anchor'
import { Connection, PublicKey, type Transaction } from '@solana/web3.js'
import { paymentAPI, type DepositConfig } from '@/api/payment'
import { useAuthStore } from '@/stores/auth'
import { useSolanaWallet } from './useSolanaWallet'
import idl from './idl/solana_deposit.json'

const CLUSTER_ENDPOINTS: Record<DepositConfig['cluster'], string> = {
  devnet: 'https://api.devnet.solana.com',
  'mainnet-beta': 'https://api.mainnet-beta.solana.com',
}

const CURRENCY_DECIMALS = { SOL: 9, USDC: 6 } as const
type Currency = keyof typeof CURRENCY_DECIMALS

const { t } = useI18n()
const authStore = useAuthStore()
const { account, signAndSendTransaction, initSolanaWallet, teardownSolanaWallet } = useSolanaWallet()

const config = ref<DepositConfig | null>(null)
const currencyOptions: Currency[] = ['SOL', 'USDC']
const currency = ref<Currency>('SOL')
const amount = ref('')
const submitting = ref(false)
const txSignature = ref('')
const errorMessage = ref('')

const parsedAmount = computed(() => {
  const value = Number(amount.value)
  return Number.isFinite(value) && value > 0 ? value : 0
})
const canSubmit = computed(() => Boolean(account.value) && parsedAmount.value > 0 && !submitting.value)

const explorerUrl = computed(() => {
  const base = `https://explorer.solana.com/tx/${txSignature.value}`
  return config.value?.cluster === 'devnet' ? `${base}?cluster=devnet` : base
})

async function buildDepositTransaction(connection: Connection, payer: PublicKey): Promise<Transaction> {
  const cfg = config.value!
  // The wallet signs via the Wallet Standard feature instead of this provider;
  // the provider only supplies the connection for account resolution.
  const providerWallet = {
    publicKey: payer,
    signTransaction: async (tx: Transaction) => tx,
    signAllTransactions: async (txs: Transaction[]) => txs,
  } as AnchorWallet
  const provider = new AnchorProvider(connection, providerWallet, { commitment: 'confirmed' })
  const program = new Program({ ...idl, address: cfg.program_id } as unknown as Idl, provider)

  const rawAmount = new BN(Math.round(parsedAmount.value * 10 ** CURRENCY_DECIMALS[currency.value]))
  const userId = new BN(authStore.user?.id ?? 0)
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const methods = program.methods as any
  if (currency.value === 'SOL') {
    return methods.depositSol(rawAmount, userId).accounts({ payer }).transaction()
  }
  return methods
    .depositUsdc(rawAmount, userId)
    .accounts({ payer, usdcMint: new PublicKey(cfg.usdc_mint) })
    .transaction()
}

async function handleSubmit(): Promise<void> {
  if (!config.value || !account.value || !canSubmit.value) return
  submitting.value = true
  errorMessage.value = ''

  try {
    const connection = new Connection(CLUSTER_ENDPOINTS[config.value.cluster], 'confirmed')
    const payer = new PublicKey(account.value.address)
    const transaction = await buildDepositTransaction(connection, payer)
    transaction.feePayer = payer
    transaction.recentBlockhash = (await connection.getLatestBlockhash()).blockhash
    txSignature.value = await signAndSendTransaction(transaction, connection)
  } catch (error) {
    console.warn('Crypto deposit failed:', error)
    errorMessage.value = t('cryptoDeposit.failed')
  } finally {
    submitting.value = false
  }
}

function resetForm(): void {
  txSignature.value = ''
  amount.value = ''
  errorMessage.value = ''
}

onMounted(async () => {
  initSolanaWallet()
  try {
    const res = await paymentAPI.getDepositConfig()
    if (res.data?.enabled) {
      config.value = res.data
    }
  } catch {
    // Deposit disabled or unavailable (404/503) — keep the card hidden.
  }
})

onBeforeUnmount(() => {
  teardownSolanaWallet()
})
</script>
