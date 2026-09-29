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

export function Register() {
  const { status, config, register } = useAuth()
  const [params] = useSearchParams()
  const next = safeNext(params.get('next'))
  const navigate = useNavigate()
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)

  if (status === 'authenticated') return <Navigate to={next} replace />
  if (status === 'loading') return <FullPageSpinner />
  if (!config?.internal.registration) return <Navigate to="/login" replace />

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const form = new FormData(event.currentTarget)
    setPending(true)
    setError(null)
    try {
      await register(
        String(form.get('email')),
        String(form.get('password')),
        String(form.get('name')),
      )
      navigate(next, { replace: true })
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setPending(false)
    }
  }

  return (
    <AuthCard title={strings.createAccountTitle}>
      <form onSubmit={submit} className="flex flex-col gap-4">
        <div className="grid gap-2">
          <Label htmlFor="name">{strings.name}</Label>
          <Input id="name" name="name" autoComplete="name" required maxLength={100} />
        </div>
        <div className="grid gap-2">
          <Label htmlFor="email">{strings.email}</Label>
          <Input id="email" name="email" type="email" autoComplete="email" required />
        </div>
        <div className="grid gap-2">
          <Label htmlFor="password">{strings.password}</Label>
          <Input
            id="password"
            name="password"
            type="password"
            autoComplete="new-password"
            required
            minLength={8}
          />
        </div>
        <Button type="submit" disabled={pending}>
          {strings.createAccount}
        </Button>
      </form>
      <FormError message={error} />
      <p className="text-muted-foreground text-center text-sm">
        {strings.haveAnAccount}{' '}
        <Link className="text-foreground underline" to={`/login?next=${encodeURIComponent(next)}`}>
          {strings.signIn}
        </Link>
      </p>
    </AuthCard>
  )
}
