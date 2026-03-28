'use client'

import { useEffect } from 'react'

export interface TxRecord {
  id: string
  type: 'sent' | 'received'
  amount: string
  asset: string
  counterparty: string
  timestamp: number
  hash?: string
  memo?: string
}

interface TxDetailSheetProps {
  tx: TxRecord
  onClose: () => void
}

export function TxDetailSheet({ tx, onClose }: TxDetailSheetProps) {
  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose() }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onClose])

  const date = new Date(tx.timestamp * 1000).toLocaleString()

  return (
    <>
      {/* Backdrop */}
      <div
        onClick={onClose}
        aria-hidden="true"
        style={{
          position: 'fixed', inset: 0, zIndex: 40,
          background: 'rgba(15,15,15,0.7)',
        }}
      />

      {/* Sheet */}
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Transaction details"
        style={{
          position: 'fixed', bottom: 0, left: 0, right: 0, zIndex: 50,
          background: 'var(--near-black)',
          borderTop: '1px solid var(--border-dim)',
          borderRadius: '1rem 1rem 0 0',
          padding: '1.25rem 1.5rem 2rem',
          maxWidth: 480,
          margin: '0 auto',
        }}
      >
        {/* Drag handle */}
        <div style={{
          width: 40, height: 4,
          background: 'rgba(246,247,248,0.15)',
          borderRadius: 2,
          margin: '0 auto 1.5rem',
        }} />

        <h3 style={{
          fontFamily: 'Lora, Georgia, serif', fontWeight: 600, fontStyle: 'italic',
          fontSize: '1.25rem', marginBottom: '1.5rem',
        }}>
          Transaction Details
        </h3>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.875rem' }}>
          <DetailRow label="Type" value={tx.type === 'sent' ? 'Sent' : 'Received'} />
          <DetailRow label="Amount" value={`${tx.amount} ${tx.asset}`} />
          <DetailRow
            label={tx.type === 'sent' ? 'To' : 'From'}
            value={`${tx.counterparty.slice(0, 8)}...${tx.counterparty.slice(-8)}`}
            mono
          />
          <DetailRow label="Date" value={date} />
          {tx.memo && <DetailRow label="Memo" value={tx.memo} />}
          {tx.hash && (
            <DetailRow
              label="Tx Hash"
              value={`${tx.hash.slice(0, 12)}...${tx.hash.slice(-12)}`}
              mono
            />
          )}
        </div>

        <button
          className="btn-ghost"
          onClick={onClose}
          style={{ marginTop: '1.75rem', width: '100%' }}
        >
          Close
        </button>
      </div>
    </>
  )
}

function DetailRow({
  label,
  value,
  mono,
}: {
  label: string
  value: string
  mono?: boolean
}) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '1rem' }}>
      <span style={{ fontSize: '0.8125rem', color: 'rgba(246,247,248,0.4)', flexShrink: 0 }}>
        {label}
      </span>
      <span style={{
        fontSize: '0.875rem',
        fontFamily: mono ? 'Inconsolata, monospace' : 'Inter, sans-serif',
        textAlign: 'right',
        wordBreak: 'break-all',
      }}>
        {value}
      </span>
    </div>
  )
}
