import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { StatusBanner } from '@/components/status/status-banner'
import type { ServiceStatus } from '@/lib/types'

const bannerCases: { status: ServiceStatus; label: string; bgClass: string }[] = [
  { status: 'operational', label: 'All Systems Operational', bgClass: 'bg-green-500' },
  { status: 'degraded_performance', label: 'Degraded System Performance', bgClass: 'bg-yellow-500' },
  { status: 'partial_outage', label: 'Partial System Outage', bgClass: 'bg-orange-500' },
  { status: 'major_outage', label: 'Major System Outage', bgClass: 'bg-red-500' },
  { status: 'under_maintenance', label: 'Scheduled Maintenance In Progress', bgClass: 'bg-blue-500' },
]

describe('StatusBanner', () => {
  it.each(bannerCases)(
    'renders label "$label" for overallStatus "$status"',
    ({ status, label }) => {
      render(<StatusBanner overallStatus={status} />)
      expect(screen.getByText(label)).toBeInTheDocument()
    },
  )

  it.each(bannerCases)(
    'applies background class "$bgClass" for overallStatus "$status"',
    ({ status, bgClass }) => {
      const { container } = render(<StatusBanner overallStatus={status} />)
      const banner = container.firstElementChild as HTMLElement
      expect(banner).toHaveClass(bgClass)
    },
  )

  it.each(bannerCases)(
    'renders an icon (svg) for overallStatus "$status"',
    ({ status }) => {
      const { container } = render(<StatusBanner overallStatus={status} />)
      const svg = container.querySelector('svg')
      expect(svg).toBeInTheDocument()
    },
  )

  it('always applies text-white class', () => {
    const { container } = render(<StatusBanner overallStatus="operational" />)
    const banner = container.firstElementChild as HTMLElement
    expect(banner).toHaveClass('text-white')
  })
})
