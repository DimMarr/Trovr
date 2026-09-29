import { useState } from 'react'
import { BrowserRouter, Route, Routes } from 'react-router'

import { Providers } from '@/components/Providers'
import { createQueryClient } from '@/lib/query'
import { strings } from '@/strings'

export function AppRoutes() {
  return (
    <Routes>
      <Route path="*" element={<h1 className="p-6 text-2xl font-semibold">{strings.appName}</h1>} />
    </Routes>
  )
}

export default function App() {
  const [queryClient] = useState(() => createQueryClient())
  return (
    <BrowserRouter>
      <Providers queryClient={queryClient}>
        <AppRoutes />
      </Providers>
    </BrowserRouter>
  )
}
