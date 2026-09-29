import { useState } from 'react'
import { BrowserRouter, Route, Routes } from 'react-router'

import { AuthProvider } from '@/auth/AuthProvider'
import { RequireAuth } from '@/auth/RequireAuth'
import { Providers } from '@/components/Providers'
import { AppLayout } from '@/components/AppLayout'
import { NotFound } from '@/components/NotFound'
import { createQueryClient } from '@/lib/query'
import { AuthCallback } from '@/routes/AuthCallback'
import { Browser } from '@/routes/Browser'
import { Login } from '@/routes/Login'
import { Register } from '@/routes/Register'
import { Shared } from '@/routes/Shared'
import { Trash } from '@/routes/Trash'

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/register" element={<Register />} />
      <Route path="/auth/callback" element={<AuthCallback />} />
      <Route element={<RequireAuth />}>
        <Route element={<AppLayout />}>
          <Route index element={<Browser />} />
          <Route path="folders/:id" element={<Browser />} />
          <Route path="shared" element={<Shared />} />
          <Route path="trash" element={<Trash />} />
          <Route path="*" element={<NotFound />} />
        </Route>
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
