import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Transaction } from '@solana/web3.js'
import {
  account,
  base58Encode,
  selectedWallet,
  signAndSendTransaction,
  type SolanaWallet,
} from '../useSolanaWallet'

vi.mock('@wallet-standard/app', () => ({
  getWallets: () => ({ get: () => [], on: () => () => {} }),
}))

const fakeAccount = {
  address: '7YWHMfk9T2hXPWbm5nLd9QpBDCAj7w4PKr2jEwLq9abc',
  publicKey: new Uint8Array(32),
  chains: ['solana:devnet'],
  features: [],
  label: undefined,
  icon: undefined,
} as const

const fakeTransaction = {
  serialize: vi.fn(() => new Uint8Array([1, 2, 3])),
} as unknown as Transaction

function walletWithFeatures(features: Record<string, unknown>): SolanaWallet {
  return {
    version: '1.0.0',
    name: 'Test Wallet',
    icon: 'data:image/png;base64,AA==',
    chains: ['solana:devnet'],
    accounts: [],
    features,
  } as unknown as SolanaWallet
}

describe('useSolanaWallet', () => {
  beforeEach(() => {
    selectedWallet.value = null
    account.value = null
    fakeTransaction.serialize = vi.fn(() => new Uint8Array([1, 2, 3]))
  })

  it('base58Encode encodes leading zero bytes', () => {
    expect(base58Encode(new Uint8Array(32))).toBe('1'.repeat(32))
    expect(base58Encode(new Uint8Array(0))).toBe('')
  })

  it('rejects when no wallet is connected', async () => {
    await expect(signAndSendTransaction(fakeTransaction)).rejects.toThrow('No Solana wallet connected')
  })

  it('prefers solana:signAndSendTransaction and returns a base58 signature', async () => {
    // Signature bytes 32 x 0x01 -> known base58 string.
    const signature = new Uint8Array(32).fill(1)
    const signAndSend = vi.fn().mockResolvedValue([{ signature }])
    selectedWallet.value = walletWithFeatures({
      'solana:signAndSendTransaction': { version: '1.0.0', signAndSendTransaction: signAndSend },
    })
    account.value = { ...fakeAccount }

    const result = await signAndSendTransaction(fakeTransaction)

    expect(signAndSend).toHaveBeenCalledOnce()
    const [input] = signAndSend.mock.calls[0]
    expect(input.chain).toBe('solana:devnet')
    expect(Array.from(input.transaction)).toEqual([1, 2, 3])
    expect(result).toBe('4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi')
  })

  it('falls back to signTransaction + sendRawTransaction', async () => {
    const signedTransaction = new Uint8Array([9, 9, 9])
    const signTransaction = vi.fn().mockResolvedValue([{ signedTransaction }])
    selectedWallet.value = walletWithFeatures({
      'solana:signTransaction': { version: '1.0.0', signTransaction },
    })
    account.value = { ...fakeAccount }
    const sendRawTransaction = vi.fn().mockResolvedValue('raw-signature')
    const connection = { sendRawTransaction } as never

    const result = await signAndSendTransaction(fakeTransaction, connection)

    expect(signTransaction).toHaveBeenCalledOnce()
    expect(sendRawTransaction).toHaveBeenCalledWith(signedTransaction)
    expect(result).toBe('raw-signature')
  })

  it('throws when the wallet cannot sign transactions', async () => {
    selectedWallet.value = walletWithFeatures({})
    account.value = { ...fakeAccount }

    await expect(signAndSendTransaction(fakeTransaction)).rejects.toThrow('cannot sign')
  })
})
