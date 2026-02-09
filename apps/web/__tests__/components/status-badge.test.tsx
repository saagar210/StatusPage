import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { StatusBadge } from '@/components/dashboard/status-badge'
import type { ServiceStatus } from '@/lib/types'

const statuses: { status: ServiceStatus; label: string; dotColor: string }[] = [
  { status: 'operational', label: 'Operational', dotColor: 'bg-green-500' },
  { status: 'degraded_performance', label: 'Degraded Performance', dotColor: 'bg-yellow-500' },
  { status: 'partial_outage', label: 'Partial Outage', dotColor: 'bg-orange-500' },
  { status: 'major_outage', label: 'Major Outage', dotColor: 'bg-red-500' },
  { status: 'under_maintenance', label: 'Under Maintenance', dotColor: 'bg-blue-500' },
]

describe('StatusBadge', () => {
  it.each(statuses)(
    'renders "$label" text for status "$status"',
    ({ status, label }) => {
      render(<StatusBadge status={status} />)
      expect(screen.getByText(label)).toBeInTheDocument()
    },
  )

  it.each(statuses)(
    'renders a dot with color class "$dotColor" for status "$status"',
    ({ status, dotColor }) => {
      const { container } = render(<StatusBadge status={status} />)
      const dot = container.querySelector('span.h-2.w-2')
      expect(dot).toBeInTheDocument()
      expect(dot).toHaveClass(dotColor)
    },
  )

  it.each(statuses)(
    'applies the correct badge style classes for status "$status"',
    ({ status }) => {
      const { container } = render(<StatusBadge status={status} />)
      const badge = container.querySelector('[data-slot="badge"]')
      expect(badge).toBeInTheDocument()
      // Badge should have the outline variant attribute
      expect(badge).toHaveAttribute('data-variant', 'outline')
    },
  )
})
