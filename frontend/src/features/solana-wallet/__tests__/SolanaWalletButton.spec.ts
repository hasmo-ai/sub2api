import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import SolanaWalletButton from '../SolanaWalletButton.vue'

const walletState = vi.hoisted(() => ({
  wallets: [] as Array<Record<string, unknown>>,
  listeners: {
    register: new Set<(...wallets: Array<Record<string, unknown>>) => void>(),
    unregister: new Set<(...wallets: Array<Record<string, unknown>>) => void>(),
  },
}))

vi.mock('@wallet-standard/app', () => ({
  getWallets: () => ({
    get: () => walletState.wallets,
    on: (event: 'register' | 'unregister', listener: (...wallets: Array<Record<string, unknown>>) => void) => {
      walletState.listeners[event].add(listener)
      return () => walletState.listeners[event].delete(listener)
    },
  }),
}))

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}))

const walletIcon = 'data:image/png;base64,AA=='

function createWallet(options: { solana?: boolean; signMessage?: boolean } = {}) {
  const account = {
    address: '7YWHMfk9T2hXPWbm5nLd9QpBDCAj7w4PKr2jEwLq9abc',
    publicKey: new Uint8Array(32),
    chains: ['solana:mainnet'],
    features: ['solana:signMessage'],
  }
  const disconnect = vi.fn().mockResolvedValue(undefined)
  const connect = vi.fn().mockResolvedValue({ accounts: [account] })

  return {
    wallet: {
      version: '1.0.0',
      name: options.solana === false ? 'Ethereum Wallet' : 'Test Solana Wallet',
      icon: walletIcon,
      chains: options.solana === false ? ['eip155:1'] : ['solana:mainnet'],
      accounts: [],
      features: {
        'standard:connect': { version: '1.0.0', connect },
        'standard:disconnect': { version: '1.0.0', disconnect },
        ...(options.signMessage === false
          ? {}
          : { 'solana:signMessage': { version: '1.0.0', signMessage: vi.fn() } }),
      },
    },
    connect,
    disconnect,
  }
}

describe('SolanaWalletButton', () => {
  beforeEach(() => {
    walletState.wallets = []
    walletState.listeners.register.clear()
    walletState.listeners.unregister.clear()
  })

  it('only lists Solana wallets that can sign a binding message', async () => {
    const compatible = createWallet()
    const wrongChain = createWallet({ solana: false })
    const cannotSign = createWallet({ signMessage: false })
    walletState.wallets = [compatible.wallet, wrongChain.wallet, cannotSign.wallet]

    const wrapper = mount(SolanaWalletButton, {
      global: { stubs: { Teleport: true } },
    })
    await wrapper.get('[data-testid="solana-wallet-trigger"]').trigger('click')

    const choices = wrapper.findAll('[data-wallet-name]')
    expect(choices).toHaveLength(1)
    expect(choices[0].attributes('data-wallet-name')).toBe('Test Solana Wallet')
  })

  it('updates the list when a wallet registers after mount', async () => {
    const wrapper = mount(SolanaWalletButton, {
      global: { stubs: { Teleport: true } },
    })
    const compatible = createWallet()
    walletState.wallets = [compatible.wallet]

    walletState.listeners.register.forEach((listener) => listener(compatible.wallet))
    await wrapper.get('[data-testid="solana-wallet-trigger"]').trigger('click')

    expect(wrapper.findAll('[data-wallet-name]')).toHaveLength(1)
  })

  it('connects, displays the account, and disconnects', async () => {
    const compatible = createWallet()
    walletState.wallets = [compatible.wallet]

    const wrapper = mount(SolanaWalletButton, {
      global: { stubs: { Teleport: true } },
    })
    await wrapper.get('[data-testid="solana-wallet-trigger"]').trigger('click')
    await wrapper.get('[data-wallet-name="Test Solana Wallet"]').trigger('click')
    await nextTick()

    expect(compatible.connect).toHaveBeenCalledOnce()
    expect(wrapper.get('[data-testid="solana-wallet-trigger"]').text()).toContain('7YWH...9abc')

    await wrapper.get('[data-testid="solana-wallet-trigger"]').trigger('click')
    await wrapper.get('[data-testid="solana-wallet-disconnect"]').trigger('click')

    expect(compatible.disconnect).toHaveBeenCalledOnce()
    expect(wrapper.get('[data-testid="solana-wallet-trigger"]').text()).toContain('solanaWallet.connect')
  })
})
