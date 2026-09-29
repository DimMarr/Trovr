import { Link } from 'react-router'

import { Button } from '@/components/ui/button'
import { strings } from '@/strings'

export function NotFound() {
  return (
    <div className="flex flex-col items-center gap-4 py-16 text-center">
      <p className="text-muted-foreground">{strings.notFound}</p>
      <Button asChild variant="outline">
        <Link to="/">{strings.backToMyFiles}</Link>
      </Button>
    </div>
  )
}
