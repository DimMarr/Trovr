import { Skeleton } from '@/components/ui/skeleton'
import { strings } from '@/strings'

export function TableSkeleton() {
  return (
    <div className="flex flex-col gap-2" role="status" aria-label={strings.loading}>
      {[0, 1, 2].map((row) => (
        <Skeleton key={row} className="h-9 w-full" />
      ))}
    </div>
  )
}
