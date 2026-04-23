import type { Book } from '@/types/book'

import { cn } from '@/lib/utils'

type BookCoverProps = {
  book: Book
  size?: 'sm' | 'md'
}

const gradients = [
  'from-cyan-700 to-teal-500',
  'from-amber-600 to-orange-500',
  'from-rose-700 to-fuchsia-600',
  'from-indigo-700 to-blue-500',
  'from-emerald-700 to-lime-500',
  'from-slate-700 to-zinc-500',
]

function initialsFromTitle(title: string): string {
  return title
    .split(' ')
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('')
}

function gradientById(bookId: number): string {
  const index = Math.abs(bookId) % gradients.length
  return gradients[index]
}

export function BookCover({ book, size = 'md' }: BookCoverProps) {
  const isSmall = size === 'sm'

  return (
    <div
      className={cn(
        'relative overflow-hidden rounded-md bg-gradient-to-br text-white shadow-sm',
        gradientById(book.id),
        isSmall ? 'h-20 w-14' : 'h-44 w-full',
      )}
      aria-hidden="true"
    >
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.24),transparent_55%)]" />
      <div className="absolute inset-x-2 bottom-2 space-y-0.5">
        <p className={cn('truncate font-semibold tracking-wide', isSmall ? 'text-[10px]' : 'text-xs')}>{book.format}</p>
        <p className={cn('font-display leading-none', isSmall ? 'text-base' : 'text-2xl')}>{initialsFromTitle(book.title)}</p>
      </div>
    </div>
  )
}
