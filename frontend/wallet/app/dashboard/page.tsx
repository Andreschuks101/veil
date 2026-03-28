'use client'

import { useEffect, useRef, useCallback, useState } from 'react'
import { useRouter } from 'next/navigation'
import { Server } from 'stellar-sdk/lib/horizon'
import { TxDetailSheet, type TxRecord } from '@/components/TxDetailSheet'

// ── Inactivity lock constant ──────────────────────────────────────────────────
const LOCK_TIMEOUT_MS = 5 * 60 * 1000

const ACTIVITY_EVENTS = ['mousemove', 'keydown', 'touchstart', 'click', 'scroll'] as const

function useInactivityLock() {
  const router          = useRouter()
  const timerRef        = useRef<ReturnType<typeof setTimeout> | null>(null)
  const lastActivityRef = useRef<number>(Date.now())

  const lock = useCallback(() => {
    sessionStorage.clear()
    router.replace('/lock')
  }, [router])

  const resetTimer = useCallback(() => {
    lastActivityRef.current = Date.now()
    if (timerRef.current) clearTimeout(timerRef.current)
    timerRef.current = setTimeout(lock, LOCK_TIMEOUT_MS)
  }, [lock])

  useEffect(() => {
    resetTimer()
    ACTIVITY_EVENTS.forEach(event =>
      window.addEventListener(event, resetTimer, { passive: true }),
    )
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current)
      ACTIVITY_EVENTS.forEach(event =>
        window.removeEventListener(event, resetTimer),
      )
    }
  }, [resetTimer])
}

// ── Types ─────────────────────────────────────────────────────────────────────

export interface WalletAsset {
  code: string
  issuer: string | null
  balance: string
}

// ── Dashboard page ────────────────────────────────────────────────────────────
export default function DashboardPage() {
  useInactivityLock()

  const router = useRouter()

  const walletAddress =
    typeof window !== 'undefined'
      ? sessionStorage.getItem('invisible_wallet_address')
      : null

  const [assets, setAssets]           = useState<WalletAsset[]>([])
  const [transactions, setTransactions] = useState<TxRecord[]>([])
  const [selectedTx, setSelectedTx]   = useState<TxRecord | null>(null)
  const [loading, setLoading]         = useState(true)

  useEffect(() => {
    if (!walletAddress) { setLoading(false); return }

    const server = new Server('https://horizon-testnet.stellar.org')

    async function fetchData() {
      try {
        // ── Balances ──────────────────────────────────────────────────────────
        const account = await server.loadAccount(walletAddress!)
        const walletAssets: WalletAsset[] = account.balances.map(b => {
          if (b.asset_type === 'native') {
            return { code: 'XLM', issuer: null, balance: b.balance }
          }
          const issued = b as { asset_code: string; asset_issuer: string; balance: string }
          return { code: issued.asset_code, issuer: issued.asset_issuer, balance: issued.balance }
        })
        setAssets(walletAssets)

        // ── Recent payment operations ─────────────────────────────────────────
        const payments = await server
          .payments()
          .forAccount(walletAddress!)
          .limit(20)
          .order('desc')
          .call()

        type HorizonPayment = {
          id: string
          type: string
          from: string
          to: string
          amount: string
          asset_type: string
          asset_code?: string
          created_at: string
          transaction_hash: string
          transaction?: { memo?: string }
        }

        const txRecords: TxRecord[] = (payments.records as HorizonPayment[])
          .filter(p => p.type === 'payment')
          .map(p => ({
            id:           p.id,
            type:         p.from === walletAddress ? 'sent' : 'received',
            amount:       p.amount,
            asset:        p.asset_type === 'native' ? 'XLM' : (p.asset_code ?? ''),
            counterparty: p.from === walletAddress ? p.to : p.from,
            timestamp:    Math.floor(new Date(p.created_at).getTime() / 1000),
            hash:         p.transaction_hash,
            memo:         p.transaction?.memo,
          }))

        setTransactions(txRecords)
      } catch {
        // Account may not yet be funded on testnet
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [walletAddress])

  return (
    <div className="wallet-shell">

      {/* Header */}
      <header className="wallet-nav">
        <span style={{
          fontFamily: 'Anton, Impact, sans-serif',
          fontSize: '1.25rem', letterSpacing: '0.08em',
          color: 'var(--gold)', userSelect: 'none',
        }}>
          VEIL
        </span>
        {walletAddress && (
          <span className="address-chip">
            {walletAddress.slice(0, 6)}…{walletAddress.slice(-6)}
          </span>
        )}
      </header>

      <main className="wallet-main" style={{ paddingTop: '3rem', paddingBottom: '3rem' }}>

        <div style={{ marginBottom: '2rem' }}>
          <h1 style={{
            fontFamily: 'Lora, Georgia, serif', fontWeight: 600, fontStyle: 'italic',
            fontSize: '1.75rem', color: 'var(--off-white)', marginBottom: '0.25rem',
          }}>
            Dashboard
          </h1>
          <p style={{ fontSize: '0.875rem', color: 'rgba(246,247,248,0.5)' }}>
            Your wallet locks automatically after 5 minutes of inactivity.
          </p>
        </div>

        {/* ── Assets section ─────────────────────────────────────────────────── */}
        <section style={{ marginBottom: '1.5rem' }}>
          <h2 style={{
            fontSize: '0.75rem', color: 'rgba(246,247,248,0.4)',
            fontFamily: 'Anton, Impact, sans-serif', letterSpacing: '0.06em',
            marginBottom: '0.75rem',
          }}>
            ASSETS
          </h2>

          {loading ? (
            <div className="card" style={{ display: 'flex', justifyContent: 'center', padding: '2rem' }}>
              <div className="spinner spinner-light" />
            </div>
          ) : assets.length === 0 ? (
            <div className="card" style={{ textAlign: 'center', padding: '2rem' }}>
              <p style={{ fontSize: '0.875rem', color: 'rgba(246,247,248,0.4)' }}>
                No assets found. Fund this address on Stellar Testnet to get started.
              </p>
            </div>
          ) : (
            <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: '0.875rem' }}>
              {assets.map(asset => (
                <div
                  key={`${asset.code}-${asset.issuer ?? 'native'}`}
                  style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}
                >
                  <div>
                    <p style={{ fontWeight: 500 }}>{asset.code}</p>
                    {asset.issuer && (
                      <p style={{ fontSize: '0.6875rem', color: 'rgba(246,247,248,0.35)', fontFamily: 'Inconsolata, monospace', marginTop: '0.125rem' }}>
                        {asset.issuer.slice(0, 6)}…{asset.issuer.slice(-6)}
                      </p>
                    )}
                  </div>
                  <span style={{ fontFamily: 'Inconsolata, monospace', fontSize: '1rem' }}>
                    {parseFloat(asset.balance).toFixed(2)}
                  </span>
                </div>
              ))}
            </div>
          )}
        </section>

        {/* Send CTA */}
        <button
          className="btn-gold"
          onClick={() => router.push('/send')}
          style={{ marginBottom: '2rem' }}
        >
          Send
        </button>

        {/* ── Activity section ───────────────────────────────────────────────── */}
        <section>
          <h2 style={{
            fontSize: '0.75rem', color: 'rgba(246,247,248,0.4)',
            fontFamily: 'Anton, Impact, sans-serif', letterSpacing: '0.06em',
            marginBottom: '0.75rem',
          }}>
            ACTIVITY
          </h2>

          {!loading && transactions.length === 0 && (
            <div className="card" style={{ textAlign: 'center', padding: '2rem' }}>
              <p style={{ fontSize: '0.875rem', color: 'rgba(246,247,248,0.4)' }}>
                No transactions yet.
              </p>
            </div>
          )}

          {transactions.length > 0 && (
            <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
              {transactions.map((tx, i) => (
                <button
                  key={tx.id}
                  onClick={() => setSelectedTx(tx)}
                  aria-label={`${tx.type === 'sent' ? 'Sent' : 'Received'} ${tx.amount} ${tx.asset}`}
                  style={{
                    display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                    width: '100%', padding: '0.875rem 1rem',
                    background: 'none', border: 'none', cursor: 'pointer',
                    borderBottom: i < transactions.length - 1 ? '1px solid var(--border-dim)' : 'none',
                    color: 'var(--off-white)', textAlign: 'left',
                    transition: 'background 100ms',
                  }}
                >
                  <div>
                    <p style={{ fontSize: '0.875rem', fontWeight: 500 }}>
                      {tx.type === 'sent' ? '↑ Sent' : '↓ Received'}
                    </p>
                    <p style={{ fontSize: '0.75rem', color: 'rgba(246,247,248,0.4)', marginTop: '0.125rem', fontFamily: 'Inconsolata, monospace' }}>
                      {tx.counterparty.slice(0, 6)}…{tx.counterparty.slice(-6)}
                    </p>
                  </div>
                  <div style={{ textAlign: 'right' }}>
                    <p style={{ fontFamily: 'Inconsolata, monospace', fontSize: '0.9375rem' }}>
                      {tx.amount} {tx.asset}
                    </p>
                  </div>
                </button>
              ))}
            </div>
          )}
        </section>

      </main>

      {selectedTx && (
        <TxDetailSheet tx={selectedTx} onClose={() => setSelectedTx(null)} />
      )}
    </div>
  )
}
