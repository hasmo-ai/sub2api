import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { applyEyamiBranding, EYAMI_BRAND } from '../eyami'
import type { PublicSettings } from '@/types'

const dir = dirname(fileURLToPath(import.meta.url))
const frontendRoot = resolve(dir, '../../..')

describe('Eyami branding', () => {
  it('applies the fork branding through public settings', () => {
    const settings = applyEyamiBranding({
      site_name: 'Sub2API',
      site_subtitle: 'Upstream',
      site_logo: '/logo.png',
      contact_info: 'support@example.com',
      doc_url: 'https://docs.example.com',
      home_content: 'Upstream content',
      version: '1.2.3'
    } as PublicSettings)

    expect(settings).toMatchObject({
      site_name: EYAMI_BRAND.name,
      site_subtitle: '',
      site_logo: EYAMI_BRAND.logo,
      contact_info: '',
      doc_url: '',
      home_content: '',
      version: '1.2.3'
    })
  })

  it('ships the icon directly and does not load a branding shim', () => {
    const indexSource = readFileSync(resolve(frontendRoot, 'index.html'), 'utf8')

    expect(indexSource).toContain(`href="${EYAMI_BRAND.icon}"`)
    expect(indexSource).not.toContain('console-branding.js')
  })
})
