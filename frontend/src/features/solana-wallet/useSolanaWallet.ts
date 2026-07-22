/**
 * Shared Solana wallet state (Wallet Standard).
 *
 * State refs live at module scope so every consumer (header button, deposit
 * card, ...) observes the same connection. Components must call
 * `initSolanaWallet()` on mount and `teardownSolanaWallet()` on unmount;
 * registry listeners are reference-counted across consumers.
 */
import { computed, shallowRef, ref } from 'vue'
import { getWallets } from '@wallet-standard/app'
import type { IdentifierString, Wallet, WalletAccount } from '@wallet-standard/base'
import {
  StandardConnect,
  StandardDisconnect,
  StandardEvents,
  type StandardConnectFeature,
  type StandardDisconnectFeature,
  type StandardEventsFeature,
} from '@wallet-standard/features'
import {
  SolanaSignAndSendTransaction,
  SolanaSignMessage,
  SolanaSignTransaction,
  type SolanaSignAndSendTransactionFeature,
  type SolanaSignMessageFeature,
  type SolanaSignTransactionFeature,
} from '@solana/wallet-standard-features'
import type { Connection, Transaction } from '@solana/web3.js'

export type SolanaWallet = Omit<Wallet, 'features'> & {
  readonly features: Wallet['features'] & StandardConnectFeature & SolanaSignMessageFeature
}

const walletRegistry = getWallets()

export const wallets = shallowRef<SolanaWallet[]>([])
export const selectedWallet = shallowRef<SolanaWallet | null>(null)
export const account = shallowRef<WalletAccount | null>(null)
export const connectingWalletName = ref('')

let removeWalletChangeListener: (() => void) | undefined
let removeRegisterListener: (() => void) | undefined
let removeUnregisterListener: (() => void) | undefined
let consumerCount = 0

export const isConnecting = computed(() => Boolean(connectingWalletName.value))
export const shortenedAddress = computed(() => {
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

/**
 * Connect to a wallet. Returns true on success; on failure returns false and
 * leaves the previous connection state untouched.
 */
export async function connect(wallet: SolanaWallet): Promise<boolean> {
  connectingWalletName.value = wallet.name

  try {
    const result = await wallet.features[StandardConnect].connect()
    const solanaAccount = result.accounts.find(isSolanaAccount)
    if (!solanaAccount) throw new Error('Wallet returned no Solana account')

    selectedWallet.value = wallet
    account.value = solanaAccount
    watchWalletAccounts(wallet)
    return true
  } catch (error) {
    console.warn('Solana wallet connection failed:', error)
    return false
  } finally {
    connectingWalletName.value = ''
  }
}

export async function disconnect(): Promise<void> {
  const wallet = selectedWallet.value

  try {
    const disconnectFeature = wallet?.features[StandardDisconnect] as StandardDisconnectFeature[typeof StandardDisconnect] | undefined
    await disconnectFeature?.disconnect()
  } catch (error) {
    console.warn('Solana wallet disconnect failed:', error)
  } finally {
    clearConnection()
  }
}

export function clearConnection(): void {
  removeWalletChangeListener?.()
  removeWalletChangeListener = undefined
  selectedWallet.value = null
  account.value = null
}

function solanaChainOf(target: WalletAccount): IdentifierString {
  return target.chains.find((chain) => chain.startsWith('solana:')) ?? 'solana:mainnet'
}

const BASE58_ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'

/** Minimal base58 encoder (avoids pulling in a bs58 dependency). */
export function base58Encode(bytes: Uint8Array): string {
  if (bytes.length === 0) return ''
  const digits = [0]
  for (const byte of bytes) {
    let carry = byte
    for (let i = 0; i < digits.length; i++) {
      carry += digits[i] << 8
      digits[i] = carry % 58
      carry = (carry / 58) | 0
    }
    while (carry > 0) {
      digits.push(carry % 58)
      carry = (carry / 58) | 0
    }
  }
  // Strip most-significant zero digits; leading zero *bytes* are handled below.
  while (digits.length > 0 && digits[digits.length - 1] === 0) digits.pop()
  let encoded = ''
  for (const byte of bytes) {
    if (byte !== 0) break
    encoded += BASE58_ALPHABET[0]
  }
  for (let i = digits.length - 1; i >= 0; i--) {
    encoded += BASE58_ALPHABET[digits[i]]
  }
  return encoded
}

/**
 * Sign and send a transaction through the connected wallet.
 *
 * Prefers the wallet's `solana:signAndSendTransaction` feature; falls back to
 * `solana:signTransaction` + `connection.sendRawTransaction` when the wallet
 * cannot send on its own (a Connection is required for the fallback).
 *
 * Returns the base58 transaction signature.
 */
export async function signAndSendTransaction(
  transaction: Transaction,
  connection?: Connection,
): Promise<string> {
  const wallet = selectedWallet.value
  const signer = account.value
  if (!wallet || !signer) throw new Error('No Solana wallet connected')

  const serialized = transaction.serialize({ requireAllSignatures: false, verifySignatures: false })
  const chain = solanaChainOf(signer)

  const sendFeature = wallet.features[SolanaSignAndSendTransaction] as
    SolanaSignAndSendTransactionFeature[typeof SolanaSignAndSendTransaction] | undefined
  if (sendFeature) {
    const [output] = await sendFeature.signAndSendTransaction({
      account: signer,
      transaction: serialized,
      chain,
    })
    if (!output) throw new Error('Wallet returned no transaction signature')
    return base58Encode(output.signature)
  }

  const signFeature = wallet.features[SolanaSignTransaction] as
    SolanaSignTransactionFeature[typeof SolanaSignTransaction] | undefined
  if (!signFeature) throw new Error('Wallet cannot sign Solana transactions')
  if (!connection) throw new Error('A Solana connection is required to send the transaction')

  const [output] = await signFeature.signTransaction({
    account: signer,
    transaction: serialized,
    chain,
  })
  if (!output) throw new Error('Wallet returned no signed transaction')
  return connection.sendRawTransaction(output.signedTransaction)
}

/** Register wallet registry listeners. Reference-counted across consumers. */
export function initSolanaWallet(): void {
  consumerCount += 1
  if (consumerCount > 1) return
  refreshWallets()
  removeRegisterListener = walletRegistry.on('register', refreshWallets)
  removeUnregisterListener = walletRegistry.on('unregister', refreshWallets)
}

/** Unregister wallet registry listeners once no consumer remains. */
export function teardownSolanaWallet(): void {
  consumerCount = Math.max(0, consumerCount - 1)
  if (consumerCount > 0) return
  removeRegisterListener?.()
  removeRegisterListener = undefined
  removeUnregisterListener?.()
  removeUnregisterListener = undefined
}

export function useSolanaWallet() {
  return {
    wallets,
    selectedWallet,
    account,
    connectingWalletName,
    isConnecting,
    shortenedAddress,
    connect,
    disconnect,
    clearConnection,
    signAndSendTransaction,
    initSolanaWallet,
    teardownSolanaWallet,
  }
}
