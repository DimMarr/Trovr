import { useState } from 'react'
import { BrowserRouter, Route, Routes } from 'react-router'

import { AuthProvider } from '@/auth/AuthProvider'
import { RequireAuth } from '@/auth/RequireAuth'
import { Providers } from '@/components/Providers'
import { UserMenu } from '@/components/UserMenu'
import { createQueryClient } from '@/lib/query'
import { AuthCallback } from '@/routes/AuthCallback'
import { Login } from '@/routes/Login'
import { Register } from '@/routes/Register'
import { strings } from '@/strings'

function Home() {
  return (
    <main className="flex flex-col gap-4 p-6">
      <h1 className="text-2xl font-semibold">{strings.appName}</h1>
      <div className="max-w-xs">
        <UserMenu />
      </div>
    </main>
  )
}

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/register" element={<Register />} />
      <Route path="/auth/callback" element={<AuthCallback />} />
      <Route element={<RequireAuth />}>
        <Route path="*" element={<Home />} />
      </Route>
    </Routes>
  )
}

export default function App() {
  const [queryClient] = useState(() => createQueryClient())
  return (
    <BrowserRouter>
      <Providers queryClient={queryClient}>
        <AuthProvider>
          <AppRoutes />
        </AuthProvider>
      </Providers>
    </BrowserRouter>
  )
}
