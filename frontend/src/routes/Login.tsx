import { type FormEvent, useState } from 'react'
import { Link, Navigate, useNavigate, useSearchParams } from 'react-router'

import { useAuth } from '@/auth/context'
import { AuthCard } from '@/components/AuthCard'
import { FormError } from '@/components/FormError'
import { FullPageSpinner } from '@/components/FullPageSpinner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { errorMessage } from '@/lib/errors'
import { safeNext } from '@/lib/navigation'
import { strings } from '@/strings'

export function Login() {
  const { status, config, loginInternal, loginOidc } = useAuth()
  const [params] = useSearchParams()
  const next = safeNext(params.get('next'))
  const navigate = useNavigate()
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)

  if (status === 'authenticated') return <Navigate to={next} replace />
  if (!config) {
    if (status === 'loading') return <FullPageSpinner />
    return (
      <AuthCard title={strings.signInTitle}>
        <FormError message={strings.errors.network_error} />
      </AuthCard>
    )
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const form = new FormData(event.currentTarget)
    setPending(true)
    setError(null)
    try {
      await loginInternal(String(form.get('email')), String(form.get('password')))
      navigate(next, { replace: true })
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setPending(false)
    }
  }

  async function sso() {
    setError(null)
    try {
      await loginOidc(next)
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  return (
    <AuthCard title={strings.signInTitle}>
      {config.internal.enabled && (
        <form onSubmit={submit} className="flex flex-col gap-4">
          <div className="grid gap-2">
            <Label htmlFor="email">{strings.email}</Label>
            <Input id="email" name="email" type="email" autoComplete="username" required />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="password">{strings.password}</Label>
            <Input
              id="password"
              name="password"
              type="password"
              autoComplete="current-password"
              required
            />
          </div>
          <Button type="submit" disabled={pending}>
            {strings.signIn}
          </Button>
        </form>
      )}
      {config.internal.enabled && config.oidc && (
        <p className="text-muted-foreground text-center text-xs uppercase">{strings.or}</p>
      )}
      {config.oidc && (
        <Button variant="outline" onClick={() => void sso()}>
          {strings.signInWithSso}
        </Button>
      )}
      {!config.internal.enabled && !config.oidc && <p>{strings.noSignInMethod}</p>}
      <FormError message={error} />
      {config.internal.registration && (
        <p className="text-muted-foreground text-center text-sm">
          {strings.noAccount}{' '}
          <Link
            className="text-foreground underline"
            to={`/register?next=${encodeURIComponent(next)}`}
          >
            {strings.createAnAccount}
          </Link>
        </p>
      )}
    </AuthCard>
  )
}
