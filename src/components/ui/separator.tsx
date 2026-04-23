import { cn } from '@/lib/utils'

function Separator({ className }: { className?: string }) {
  return <div className={cn('h-px w-full bg-border', className)} role="separator" aria-hidden="true" />
}

export { Separator }
