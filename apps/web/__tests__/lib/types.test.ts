import { describe, it, expect } from 'vitest'
import {
  SERVICE_STATUS_LABELS,
  INCIDENT_STATUS_LABELS,
  INCIDENT_IMPACT_LABELS,
  INCIDENT_IMPACT_COLORS,
  PLAN_MONITOR_LIMITS,
  formatPlanMonitorLimit,
} from '@/lib/types'
import type {
  ServiceStatus,
  IncidentStatus,
  IncidentImpact,
  OrganizationPlan,
} from '@/lib/types'

const SERVICE_STATUSES: ServiceStatus[] = [
  'operational',
  'degraded_performance',
  'partial_outage',
  'major_outage',
  'under_maintenance',
]

const INCIDENT_STATUSES: IncidentStatus[] = [
  'investigating',
  'identified',
  'monitoring',
  'resolved',
]

const INCIDENT_IMPACTS: IncidentImpact[] = [
  'none',
  'minor',
  'major',
  'critical',
]

describe('SERVICE_STATUS_LABELS', () => {
  it('has an entry for every ServiceStatus value', () => {
    for (const status of SERVICE_STATUSES) {
      expect(SERVICE_STATUS_LABELS[status]).toBeDefined()
      expect(typeof SERVICE_STATUS_LABELS[status]).toBe('string')
      expect(SERVICE_STATUS_LABELS[status].length).toBeGreaterThan(0)
    }
  })

  it('has exactly the right number of keys', () => {
    expect(Object.keys(SERVICE_STATUS_LABELS)).toHaveLength(SERVICE_STATUSES.length)
  })
})

describe('INCIDENT_STATUS_LABELS', () => {
  it('has an entry for every IncidentStatus value', () => {
    for (const status of INCIDENT_STATUSES) {
      expect(INCIDENT_STATUS_LABELS[status]).toBeDefined()
      expect(typeof INCIDENT_STATUS_LABELS[status]).toBe('string')
      expect(INCIDENT_STATUS_LABELS[status].length).toBeGreaterThan(0)
    }
  })

  it('has exactly the right number of keys', () => {
    expect(Object.keys(INCIDENT_STATUS_LABELS)).toHaveLength(INCIDENT_STATUSES.length)
  })
})

describe('INCIDENT_IMPACT_LABELS', () => {
  it('has an entry for every IncidentImpact value', () => {
    for (const impact of INCIDENT_IMPACTS) {
      expect(INCIDENT_IMPACT_LABELS[impact]).toBeDefined()
      expect(typeof INCIDENT_IMPACT_LABELS[impact]).toBe('string')
      expect(INCIDENT_IMPACT_LABELS[impact].length).toBeGreaterThan(0)
    }
  })

  it('has exactly the right number of keys', () => {
    expect(Object.keys(INCIDENT_IMPACT_LABELS)).toHaveLength(INCIDENT_IMPACTS.length)
  })
})

describe('INCIDENT_IMPACT_COLORS', () => {
  it('has an entry for every IncidentImpact value', () => {
    for (const impact of INCIDENT_IMPACTS) {
      expect(INCIDENT_IMPACT_COLORS[impact]).toBeDefined()
      expect(typeof INCIDENT_IMPACT_COLORS[impact]).toBe('string')
      expect(INCIDENT_IMPACT_COLORS[impact].length).toBeGreaterThan(0)
    }
  })

  it('has exactly the right number of keys', () => {
    expect(Object.keys(INCIDENT_IMPACT_COLORS)).toHaveLength(INCIDENT_IMPACTS.length)
  })
})


const ORGANIZATION_PLANS: OrganizationPlan[] = [
  'free',
  'pro',
  'team',
]

describe('PLAN_MONITOR_LIMITS', () => {
  it('has an entry for every OrganizationPlan value', () => {
    for (const plan of ORGANIZATION_PLANS) {
      expect(PLAN_MONITOR_LIMITS[plan]).not.toBeUndefined()
    }
  })

  it('formats monitor limits correctly', () => {
    expect(formatPlanMonitorLimit('free')).toBe('3')
    expect(formatPlanMonitorLimit('pro')).toBe('20')
    expect(formatPlanMonitorLimit('team')).toBe('Unlimited')
  })
})
