'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { Keypair } from '@stellar/stellar-sdk'
import { VeilLogo } from '@/components/VeilLogo'
import { OnboardingTutorial } from '@/components/OnboardingTutorial'
import { useInvisibleWallet } from '@veil/sdk'

const CONFIG = {
  rpcUrl: 'https://soroban-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015',
  factoryAddress: process.env.NEXT_PUBLIC_FACTORY_CONTRACT_ID ?? '',
}

type Step = 'landing' | 'registering' | 'deploying' | 'done'

export default function OnboardingPage() {
  const router = useRouter()
  const [step, setStep] = useState<Step>('landing')
  const [error, setError] = useState<string | null>(null)
  const [address, setAddress] = useState<string | null>(null)
  const [showTutorial, setShowTutorial] = useState(false)

  useEffect(() => {
    // If a wallet already exists, go straight to the lock/unlock screen
    const existingWallet = localStorage.getItem('invisible_wallet_address')
    if (existingWallet) {
      router.replace('/lock')
      return
    }
    const seen = localStorage.getItem('veil_seen_tutorial')
    if (!seen) {
      setShowTutorial(true)
    }
  }, [])

  const handleTutorialComplete = () => {
    localStorage.setItem('veil_seen_tutorial', '1')
    setShowTutorial(false)
  }

  const wallet = useInvisibleWallet(CONFIG)

  async function handleCreate() {
    setError(null)
    setStep('registering')
    let success = false
    try {
      const result = await wallet.register()
      if (!result) throw new Error('Registration returned no result')

      setStep('deploying')
      const signerKeypair = Keypair.random()
      const signerSecret  = signerKeypair.secret()

      // Fund the fee-payer keypair via Friendbot before deploying on testnet
      const friendbotRes = await fetch(
        `https://friendbot.stellar.org/?addr=${signerKeypair.publicKey()}`
      )
      if (!friendbotRes.ok) throw new Error('Friendbot funding failed — try again')

      // Pass secret string so the SDK uses its own Keypair instance internally,
      // avoiding XDR type mismatches between two stellar-sdk copies.
      const deployed = await wallet.deploy(signerSecret)

      // Persist minimal session to sessionStorage for the dashboard
      sessionStorage.setItem('invisible_wallet_address', deployed.walletAddress)
      sessionStorage.setItem('veil_signer_secret', signerSecret)

      setAddress(deployed.walletAddress)
      setStep('done')
      success = true
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      setError(msg)
      setStep('landing')
    }

    // Navigate outside try/catch so a routing error can't reset the page to 'landing'
    if (success) {
      await new Promise(r => setTimeout(r, 1200))
      router.push('/dashboard')
    }
  }

  function handleContinue() {
    router.push('/dashboard')
  }

  return (
    <>
      {showTutorial && <OnboardingTutorial onComplete={handleTutorialComplete} />}
      <div className="wallet-shell" style={{ justifyContent: 'center', alignItems: 'center', padding: '2rem 1.25rem', minHeight: '100dvh' }}>
      <div style={{ maxWidth: 400, width: '100%' }}>

        {/* Logo + wordmark */}
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1rem', marginBottom: '3rem' }}>
          <div style={{ position: 'relative' }} className="biometric-pulse">
            <VeilLogo size={64} />
          </div>
          <div style={{ textAlign: 'center' }}>
            <h1 style={{ fontFamily: 'Anton, Impact, sans-serif', fontSize: '2.5rem', letterSpacing: '0.08em', color: 'var(--gold)' }}>
              VEIL
            </h1>
            <p style={{ fontSize: '0.9375rem', color: 'rgba(246,247,248,0.5)', marginTop: '0.25rem' }}>
              Your passkey is your wallet
            </p>
          </div>
        </div>

        {/* Main card */}
        {step === 'landing' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
            <button className="btn-gold" onClick={handleCreate}>
              Create wallet
            </button>
            <button className="btn-ghost" onClick={() => router.push('/recover')}>
              Recover existing wallet
            </button>
            {error && (
              <p style={{ fontSize: '0.8125rem', color: 'var(--teal)', textAlign: 'center', marginTop: '0.5rem' }}>
                {error}
              </p>
            )}
          </div>
        )}

        {(step === 'registering' || step === 'deploying') && (
          <div className="card" style={{ textAlign: 'center' }}>
            <div style={{ display: 'flex', justifyContent: 'center', marginBottom: '1rem' }}>
              <div className="spinner spinner-light" />
            </div>
            <p style={{ fontFamily: 'Inter', fontWeight: 500, color: 'var(--off-white)' }}>
              {step === 'registering' ? 'Waiting for biometric...' : 'Deploying wallet on-chain...'}
            </p>
            <p style={{ fontSize: '0.8125rem', color: 'rgba(246,247,248,0.4)', marginTop: '0.5rem' }}>
              {step === 'registering'
                ? 'Approve the passkey prompt on your device'
                : 'Broadcasting to Stellar Testnet'}
            </p>
          </div>
        )}

        {step === 'done' && address && (
          <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            <div style={{ textAlign: 'center' }}>
              <svg width="40" height="40" viewBox="0 0 40 40" fill="none" style={{ margin: '0 auto 0.75rem' }}>
                <circle cx="20" cy="20" r="19" stroke="var(--teal)" strokeWidth="1.5" />
                <path d="M13 20.5l5 5 9-9" stroke="var(--teal)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
              <p style={{ fontFamily: 'Lora, Georgia, serif', fontWeight: 600, fontStyle: 'italic', fontSize: '1.25rem', color: 'var(--off-white)' }}>
                Wallet created
              </p>
            </div>

            <div>
              <p style={{ fontSize: '0.75rem', color: 'rgba(246,247,248,0.4)', marginBottom: '0.5rem', fontFamily: 'Inter' }}>
                YOUR WALLET ADDRESS
              </p>
              <div className="address-chip" style={{ width: '100%', justifyContent: 'center', borderRadius: 12, padding: '0.75rem 1rem' }}>
                {address.slice(0, 8)}...{address.slice(-8)}
              </div>
            </div>

            <button className="btn-gold" onClick={handleContinue}>
              Open wallet
            </button>
          </div>
        )}

        {/* Footer */}
        <p style={{ textAlign: 'center', fontSize: '0.75rem', color: 'rgba(246,247,248,0.25)', marginTop: '2rem' }}>
          No seed phrase. No private key. Powered by{' '}
          <span style={{ color: 'rgba(246,247,248,0.4)' }}>Stellar Soroban</span>
        </p>
      </div>
    </div>
    </>
  )
}
