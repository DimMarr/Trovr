import { LogOutIcon } from 'lucide-react'

import { useAuth } from '@/auth/context'
import { Button } from '@/components/ui/button'
import { strings } from '@/strings'

/** The signed-in user and a sign-out button. */
export function UserMenu() {
  const { user, logout } = useAuth()
  if (!user) return null
  return (
    <div className="flex items-center gap-2">
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{user.display_name}</p>
        <p className="text-muted-foreground truncate text-xs">{user.email}</p>
      </div>
      <Button
        variant="ghost"
        size="icon"
        onClick={() => void logout()}
        aria-label={strings.signOut}
      >
        <LogOutIcon />
      </Button>
    </div>
  )
}
