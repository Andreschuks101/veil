'use client'

import { useEffect, useRef, useCallback, useState } from 'react'
import { useRouter } from 'next/navigation'

// ── Inactivity lock constant ──────────────────────────────────────────────────
// After this many milliseconds of no user interaction the wallet is locked and
// sessionStorage is cleared. Set to 5 minutes per the security spec.
const LOCK_TIMEOUT_MS = 5 * 60 * 1000

// Events that count as user activity and reset the inactivity timer
const ACTIVITY_EVENTS = ['mousemove', 'keydown', 'touchstart', 'click', 'scroll'] as const

// ── useInactivityLock ─────────────────────────────────────────────────────────
function useInactivityLock() {
  const router          = useRouter()
  const timerRef        = useRef<ReturnType<typeof setTimeout> | null>(null)
  const lastActivityRef = useRef<number>(Date.now())

  const lock = useCallback(() => {
    // Clear the session — user must re-authenticate via passkey
    sessionStorage.clear()
    router.replace('/lock')
  }, [router])

  const resetTimer = useCallback(() => {
    lastActivityRef.current = Date.now()

    if (timerRef.current) clearTimeout(timerRef.current)

    timerRef.current = setTimeout(lock, LOCK_TIMEOUT_MS)
  }, [lock])

  useEffect(() => {
    // Start the timer immediately on mount
    resetTimer()

    // Attach activity listeners — each resets the countdown
    ACTIVITY_EVENTS.forEach((event) =>
      window.addEventListener(event, resetTimer, { passive: true }),
    )

    return () => {
      // Clean up on unmount
      if (timerRef.current) clearTimeout(timerRef.current)
      ACTIVITY_EVENTS.forEach((event) =>
        window.removeEventListener(event, resetTimer),
      )
    }
  }, [resetTimer])
}

// ── Dashboard page ────────────────────────────────────────────────────────────
export default function DashboardPage() {
  // Activate inactivity lock for the entire dashboard session
  useInactivityLock()

  const [walletAddress, setWalletAddress] = useState<string | null>(null)
  const [balance, setBalance] = useState<string | null>(null)
  const [isFunding, setIsFunding] = useState(false)
  const [fundingError, setFundingError] = useState<string | null>(null)

  const isTestnet = process.env.NEXT_PUBLIC_NETWORK === 'testnet'

  // Load wallet address on mount
  useEffect(() => {
    const stored = sessionStorage.getItem('invisible_wallet_address')
    setWalletAddress(stored)
  }, [])

  const fetchBalance = useCallback(async () => {
    if (!walletAddress) return

    const horizonUrl = isTestnet
      ? 'https://horizon-testnet.stellar.org'
      : 'https://horizon.stellar.org'

    try {
      const res = await fetch(`${horizonUrl}/accounts/${walletAddress}`)
      if (res.status === 404) {
        setBalance('0')
        return
      }
      if (res.ok) {
        const data = await res.json()
        const native = data.balances.find((b: any) => b.asset_type === 'native')
        setBalance(native?.balance || '0')
      }
    } catch (err) {
      console.error('Balance fetch failed', err)
    }
  }, [walletAddress, isTestnet])

  useEffect(() => {
    fetchBalance()
  }, [fetchBalance])

  const handleFund = async () => {
    if (!walletAddress) return
    setIsFunding(true)
    setFundingError(null)
    try {
      const res = await fetch(`https://friendbot.stellar.org/?addr=${walletAddress}`)
      if (!res.ok) throw new Error('Friendbot failed')
      await new Promise((r) => setTimeout(r, 2000)) // Wait for ledger close ingestion
      await fetchBalance()
    } catch (err) {
      setFundingError('Funding failed. Please try again.')
    } finally {
      setIsFunding(false)
    }
  }

  return (
    <div className="wallet-shell">

      {/* ── Header — .wallet-nav from globals.css ── */}
      <header className="wallet-nav">
        {/* Wordmark — Anton ALL CAPS per Stellar brand manual */}
        <span style={{ fontFamily: 'Anton, Impact, sans-serif', fontSize: '1.25rem', letterSpacing: '0.08em', color: 'var(--gold)', userSelect: 'none' }}>
          VEIL
        </span>
        {/* Wallet address chip — Inconsolata font per brand */}
        {walletAddress && (
          <span className="address-chip">
            {walletAddress.slice(0, 6)}…{walletAddress.slice(-6)}
          </span>
        )}
      </header>

      {/* ── Main content — .wallet-main from globals.css ── */}
      <main className="wallet-main" style={{ paddingTop: '3rem', paddingBottom: '3rem' }}>

        <div style={{ marginBottom: '2rem' }}>
          {/* Heading — Lora SemiBold Italic per brand */}
          <h1 style={{ fontFamily: 'Lora, Georgia, serif', fontWeight: 600, fontStyle: 'italic', fontSize: '1.75rem', color: 'var(--off-white)', marginBottom: '0.25rem' }}>
            Dashboard
          </h1>
          <p style={{ fontSize: '0.875rem', color: 'rgba(246,247,248,0.5)' }}>
            Your wallet locks automatically after 5 minutes of inactivity.
          </p>
        </div>

        {/* ── Balance Display ── */}
        <div style={{ marginBottom: '3rem' }}>
          <p style={{ fontSize: '0.75rem', fontFamily: 'Anton, Impact, sans-serif', color: 'var(--warm-grey)', letterSpacing: '0.08em', marginBottom: '0.5rem' }}>
            AVAILABLE BALANCE
          </p>
          <div style={{ fontFamily: 'Lora, Georgia, serif', fontWeight: 600, fontStyle: 'italic', fontSize: '2.5rem', color: 'var(--off-white)' }}>
            {balance !== null 
              ? `${parseFloat(balance).toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 7 })} XLM` 
              : '—'
            }
          </div>

          {/* Faucet button for zero-balance testnet wallets */}
          {isTestnet && balance === '0' && (
            <div style={{ marginTop: '1.25rem' }}>
              <button
                className="btn-ghost"
                onClick={handleFund}
                disabled={isFunding}
                style={{ width: 'auto', paddingLeft: '1.5rem', paddingRight: '1.5rem', minHeight: '3rem' }}
              >
                {isFunding ? (
                  <div className="spinner spinner-light" style={{ width: '1.25rem', height: '1.25rem' }} />
                ) : (
                  'Fund with testnet XLM'
                )}
              </button>

              {fundingError && (
                <p style={{ color: 'var(--teal)', fontSize: '0.75rem', marginTop: '0.75rem' }}>
                  {fundingError}
                </p>
              )}
            </div>
          )}
        </div>

        {/* Placeholder — replace with real wallet UI */}
        <div className="card" style={{ textAlign: 'center', padding: '3rem 2rem' }}>
          <p style={{ fontSize: '0.875rem', color: 'rgba(246,247,248,0.4)' }}>
            Wallet content goes here.
          </p>
        </div>

      </main>
    </div>
  )
}
