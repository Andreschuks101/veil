'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { VeilLogo } from '@/components/VeilLogo'
import { useInvisibleWallet } from '@veil/sdk'

const CONFIG = {
  rpcUrl: 'https://soroban-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015',
  factoryAddress: process.env.NEXT_PUBLIC_FACTORY_CONTRACT_ID ?? '',
}

type Step = 'idle' | 'authenticating' | 'done' | 'error'

export default function RecoverPage() {
  const router = useRouter()
  const [step, setStep] = useState<Step>('idle')
  const [error, setError] = useState<string | null>(null)

  const wallet = useInvisibleWallet(CONFIG)

  async function handleRecover() {
    setError(null)
    setStep('authenticating')
    try {
      // login() triggers a WebAuthn assertion and returns the public key + address
      const result = await wallet.login()
      if (!result?.walletAddress) throw new Error('Could not derive wallet address from passkey')

      // The signer secret stored during initial wallet creation is needed to authorise
      // future transactions. login() re-derives the wallet address from the passkey but
      // cannot recover the original signer secret — the user must have it persisted.
      // For now we restore the wallet address so the dashboard loads correctly.
      sessionStorage.setItem('invisible_wallet_address', result.walletAddress)

      setStep('done')
      setTimeout(() => router.push('/dashboard'), 800)
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err))
      setStep('error')
    }
  }

  return (
    <div className="wallet-shell" style={{ justifyContent: 'center', alignItems: 'center', padding: '2rem 1.25rem', minHeight: '100dvh' }}>
      <div style={{ maxWidth: 400, width: '100%' }}>

        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1rem', marginBottom: '3rem' }}>
          <VeilLogo size={48} />
          <div style={{ textAlign: 'center' }}>
            <h1 style={{ fontFamily: 'Lora, Georgia, serif', fontWeight: 600, fontStyle: 'italic', fontSize: '1.75rem' }}>
              Recover wallet
            </h1>
            <p style={{ fontSize: '0.875rem', color: 'rgba(246,247,248,0.4)', marginTop: '0.375rem' }}>
              Authenticate with your existing passkey
            </p>
          </div>
        </div>

        {step === 'idle' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
            <button className="btn-gold" onClick={handleRecover}>
              Use passkey to recover
            </button>
            <button className="btn-ghost" onClick={() => router.push('/')}>
              Back
            </button>
            {error && (
              <p style={{ fontSize: '0.8125rem', color: 'var(--teal)', textAlign: 'center', marginTop: '0.5rem' }}>
                {error}
              </p>
            )}
          </div>
        )}

        {step === 'authenticating' && (
          <div className="card" style={{ textAlign: 'center' }}>
            <div style={{ display: 'flex', justifyContent: 'center', marginBottom: '1rem' }}>
              <div className="spinner spinner-light" />
            </div>
            <p style={{ fontWeight: 500 }}>Waiting for passkey...</p>
            <p style={{ fontSize: '0.8125rem', color: 'rgba(246,247,248,0.4)', marginTop: '0.5rem' }}>
              Approve the prompt on your device
            </p>
          </div>
        )}

        {step === 'done' && (
          <div className="card" style={{ textAlign: 'center' }}>
            <svg width="40" height="40" viewBox="0 0 40 40" fill="none" style={{ margin: '0 auto 0.75rem' }}>
              <circle cx="20" cy="20" r="19" stroke="var(--teal)" strokeWidth="1.5" />
              <path d="M13 20.5l5 5 9-9" stroke="var(--teal)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            <p style={{ fontWeight: 500 }}>Wallet recovered</p>
          </div>
        )}

        {step === 'error' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
            <div className="card" style={{ textAlign: 'center' }}>
              <p style={{ fontWeight: 500, marginBottom: '0.5rem' }}>Recovery failed</p>
              <p style={{ fontSize: '0.8125rem', color: 'rgba(246,247,248,0.4)' }}>{error}</p>
            </div>
            <button className="btn-ghost" onClick={() => setStep('idle')}>
              Try again
            </button>
          </div>
        )}
      </div>
    </div>
  )
}
