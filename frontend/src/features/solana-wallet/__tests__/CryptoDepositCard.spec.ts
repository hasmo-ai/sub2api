import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import CryptoDepositCard from '../CryptoDepositCard.vue'

const mocks = await vi.hoisted(async () => {
  const { ref } = await import('vue')
  const account = ref<{
    address: string
    publicKey: Uint8Array
    chains: string[]
    features: string[]
  } | null>(null)
  const transaction = { feePayer: null, recentBlockhash: null }
  const builder = () => ({
    accounts: vi.fn(() => ({ transaction: vi.fn().mockResolvedValue(transaction) })),
  })
  return {
    account,
    transaction,
    getDepositConfig: vi.fn(),
    signAndSendTransaction: vi.fn().mockResolvedValue('5ignature111111111111111111111111111111111111111111111111111'),
    depositSol: vi.fn(builder),
    depositUsdc: vi.fn(builder),
    getLatestBlockhash: vi.fn().mockResolvedValue({ blockhash: 'blockhash123' }),
  }
})

vi.mock('@/api/payment', () => ({
  paymentAPI: {
    getDepositConfig: mocks.getDepositConfig,
  },
}))

vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({ user: { id: 42 } }),
}))

vi.mock('../useSolanaWallet', () => ({
  useSolanaWallet: () => ({
    account: mocks.account,
    signAndSendTransaction: mocks.signAndSendTransaction,
    initSolanaWallet: vi.fn(),
    teardownSolanaWallet: vi.fn(),
  }),
}))

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string, params?: Record<string, unknown>) => (
    params ? `${key}:${JSON.stringify(params)}` : key
  ) }),
}))

vi.mock('@coral-xyz/anchor', () => ({
  AnchorProvider: class {
    constructor(
      public connection: unknown,
      public wallet: unknown,
      public opts: unknown,
    ) {}
  },
  BN: class {
    constructor(public value: unknown) {}
  },
  Program: class {
    methods = {
      depositSol: mocks.depositSol,
      depositUsdc: mocks.depositUsdc,
    }
    constructor(public idl: unknown, public provider: unknown) {}
  },
}))

vi.mock('@solana/web3.js', () => ({
  Connection: class {
    getLatestBlockhash = mocks.getLatestBlockhash
    constructor(public endpoint: string, public commitment: string) {}
  },
  PublicKey: class {
    constructor(public value: string) {}
  },
}))

const enabledConfig = {
  enabled: true,
  cluster: 'devnet',
  program_id: 'DepoSit1111111111111111111111111111111111111',
  usdc_mint: 'USDCmint111111111111111111111111111111111111',
  min_deposit_usd: 1,
}

const connectedAccount = {
  address: '7YWHMfk9T2hXPWbm5nLd9QpBDCAj7w4PKr2jEwLq9abc',
  publicKey: new Uint8Array(32),
  chains: ['solana:devnet'],
  features: ['solana:signAndSendTransaction'],
}

describe('CryptoDepositCard', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.account.value = null
    mocks.getDepositConfig.mockResolvedValue({ data: enabledConfig })
  })

  it('renders nothing when on-chain deposit is disabled', async () => {
    mocks.getDepositConfig.mockResolvedValue({ data: { ...enabledConfig, enabled: false } })
    const wrapper = mount(CryptoDepositCard)
    await flushPromises()

    expect(wrapper.find('[data-testid="crypto-deposit-card"]').exists()).toBe(false)
  })

  it('renders nothing when the config request fails', async () => {
    mocks.getDepositConfig.mockRejectedValue({ status: 404 })
    const wrapper = mount(CryptoDepositCard)
    await flushPromises()

    expect(wrapper.find('[data-testid="crypto-deposit-card"]').exists()).toBe(false)
  })

  it('renders the card with config and asks to connect a wallet', async () => {
    const wrapper = mount(CryptoDepositCard)
    await flushPromises()

    expect(wrapper.get('[data-testid="crypto-deposit-card"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('cryptoDeposit.title')
    expect(wrapper.text()).toContain('cryptoDeposit.minHint:{"usd":1}')
    expect(wrapper.find('[data-currency="SOL"]').exists()).toBe(true)
    expect(wrapper.find('[data-currency="USDC"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="crypto-deposit-connect-hint"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="crypto-deposit-submit"]').exists()).toBe(false)
  })

  it('submits a SOL deposit through the connected wallet', async () => {
    mocks.account.value = connectedAccount
    const wrapper = mount(CryptoDepositCard)
    await flushPromises()

    await wrapper.get('[data-testid="crypto-deposit-amount"]').setValue('0.5')
    await wrapper.get('[data-testid="crypto-deposit-submit"]').trigger('click')
    await flushPromises()

    expect(mocks.depositSol).toHaveBeenCalledOnce()
    expect(mocks.depositUsdc).not.toHaveBeenCalled()
    expect(mocks.signAndSendTransaction).toHaveBeenCalledOnce()
    const [sentTransaction] = mocks.signAndSendTransaction.mock.calls[0]
    expect(sentTransaction.recentBlockhash).toBe('blockhash123')
    expect(wrapper.get('[data-testid="crypto-deposit-success"]').text()).toContain('5ignature')
    expect(wrapper.get('a').attributes('href')).toContain('?cluster=devnet')
  })

  it('submits a USDC deposit with the configured mint', async () => {
    mocks.account.value = connectedAccount
    const wrapper = mount(CryptoDepositCard)
    await flushPromises()

    await wrapper.get('[data-currency="USDC"]').trigger('click')
    await wrapper.get('[data-testid="crypto-deposit-amount"]').setValue('10')
    await wrapper.get('[data-testid="crypto-deposit-submit"]').trigger('click')
    await flushPromises()

    expect(mocks.depositUsdc).toHaveBeenCalledOnce()
    expect(mocks.depositSol).not.toHaveBeenCalled()
    expect(mocks.signAndSendTransaction).toHaveBeenCalledOnce()
  })

  it('shows an error when the wallet rejects the transaction', async () => {
    mocks.account.value = connectedAccount
    mocks.signAndSendTransaction.mockRejectedValueOnce(new Error('User rejected'))
    const wrapper = mount(CryptoDepositCard)
    await flushPromises()

    await wrapper.get('[data-testid="crypto-deposit-amount"]').setValue('1')
    await wrapper.get('[data-testid="crypto-deposit-submit"]').trigger('click')
    await flushPromises()

    expect(wrapper.get('[data-testid="crypto-deposit-error"]').text()).toContain('cryptoDeposit.failed')
    expect(wrapper.find('[data-testid="crypto-deposit-success"]').exists()).toBe(false)
  })
})
