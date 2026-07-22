import type { PublicSettings } from '@/types'

export const EYAMI_BRAND = {
  name: 'Eyami',
  subtitle: '',
  logo: '/eyami-branding/eyami-wordmark.svg',
  icon: '/eyami-branding/eyami-icon.svg'
} as const

export function applyEyamiBranding(settings: PublicSettings): PublicSettings {
  return {
    ...settings,
    site_name: EYAMI_BRAND.name,
    site_subtitle: EYAMI_BRAND.subtitle,
    site_logo: EYAMI_BRAND.logo,
    contact_info: '',
    doc_url: '',
    home_content: ''
  }
}
