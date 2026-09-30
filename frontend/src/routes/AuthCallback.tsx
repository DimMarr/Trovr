import { useEffect, useRef, useState } from 'react'
import { Link, useNavigate } from 'react-router'

import { useAuth } from '@/auth/context'
import { AuthCard } from '@/components/AuthCard'
import { FormError } from '@/components/FormError'
import { FullPageSpinner } from '@/components/FullPageSpinner'
import { safeNext } from '@/lib/navigation'
import { strings } from '@/strings'

/** Where the identity provider sends the user back after an OIDC sign-in. */
export function AuthCallback() {
  const { config, completeOidc } = useAuth()
  const navigate = useNavigate()
  const [failed, setFailed] = useState(false)
  // The authorization code is single-use: never complete twice (StrictMode runs effects twice).
  const started = useRef(false)

  useEffect(() => {
    if (!config || started.current) return
    started.current = true
    completeOidc()
      .then((next) => navigate(safeNext(next), { replace: true }))
      .catch(() => setFailed(true))
  }, [config, completeOidc, navigate])

  if (!failed) return <FullPageSpinner />
  return (
    <AuthCard title={strings.signInTitle}>
      <FormError message={strings.signInFailed} />
      <Link className="text-sm underline" to="/login">
        {strings.backToSignIn}
      </Link>
    </AuthCard>
  )
}
