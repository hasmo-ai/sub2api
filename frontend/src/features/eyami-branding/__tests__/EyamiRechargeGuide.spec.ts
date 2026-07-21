import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import EyamiRechargeGuide from '../EyamiRechargeGuide.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, string>) =>
      params ? `${key}:${params.price}:${params.credit}` : key
  })
}))

describe('EyamiRechargeGuide', () => {
  it('renders every recharge option as a safe external link', () => {
    const wrapper = mount(EyamiRechargeGuide)
    const links = wrapper.findAll('a')

    expect(links).toHaveLength(4)
    expect(links.map((link) => link.attributes('href'))).toEqual([
      'https://pay.ldxp.cn/item/yrhxzl',
      'https://pay.ldxp.cn/item/v3gmy3',
      'https://pay.ldxp.cn/item/ssoboq',
      'https://pay.ldxp.cn/item/dm6ylc'
    ])
    expect(links.every((link) => link.attributes('rel') === 'noopener noreferrer')).toBe(true)
    expect(wrapper.text()).toContain('¥100')
    expect(wrapper.text()).toContain('$200')
  })
})
