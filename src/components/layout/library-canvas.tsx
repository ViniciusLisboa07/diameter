import { memo, useCallback } from 'react'

import type { Book } from '@/types/book'

import { BookCover } from '@/components/layout/book-cover'
import type { LibraryViewMode } from '@/components/layout/top-bar'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent } from '@/components/ui/card'
import { cn } from '@/lib/utils'

type LibraryCanvasProps = {
  books: Book[]
  selectedBookId: number
  viewMode: LibraryViewMode
  onSelectBook: (bookId: number) => void
}

type LibraryBookItemProps = {
  book: Book
  isSelected: boolean
  viewMode: LibraryViewMode
  onSelectBook: (bookId: number) => void
}

const LibraryBookItem = memo(function LibraryBookItem({ book, isSelected, viewMode, onSelectBook }: LibraryBookItemProps) {
  const handleSelectBook = useCallback(() => onSelectBook(book.id), [book.id, onSelectBook])

  if (viewMode === 'list') {
    return (
      <Card
        className={cn(
          'library-item library-item-list cursor-pointer border transition-colors hover:border-primary/50',
          isSelected && 'border-primary ring-2 ring-primary/20',
        )}
        onClick={handleSelectBook}
      >
        <CardContent className="flex min-w-0 items-center gap-4 p-3 sm:p-4">
          <BookCover book={book} size="sm" />

          <div className="min-w-0 flex-1 space-y-1">
            <p className="truncate text-base font-semibold">{book.title}</p>
            <p className="truncate text-sm text-muted-foreground">{book.author}</p>
          </div>

          <div className="flex shrink-0 items-center gap-2">
            <Badge variant="secondary">{book.format}</Badge>
          </div>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card
      className={cn(
        'library-item cursor-pointer border transition-colors hover:border-primary/50',
        isSelected && 'border-primary ring-2 ring-primary/20',
      )}
      onClick={handleSelectBook}
    >
      <CardContent className="space-y-4 p-4">
        <BookCover book={book} />
        <div className="space-y-1">
          <p className="line-clamp-1 text-base font-semibold">{book.title}</p>
          <p className="line-clamp-1 text-sm text-muted-foreground">{book.author}</p>
        </div>
        <div className="flex items-center justify-between gap-2">
          <Badge variant="secondary">{book.format}</Badge>
          <span className="text-xs text-muted-foreground">{book.year}</span>
        </div>
      </CardContent>
    </Card>
  )
})

export function LibraryCanvas({ books, selectedBookId, viewMode, onSelectBook }: LibraryCanvasProps) {
  if (viewMode === 'list') {
    return (
      <main className="library-scroll flex h-full min-w-0 flex-col gap-3 overflow-x-hidden overflow-y-auto pb-2">
        {books.map((book) => (
          <LibraryBookItem
            key={book.id}
            book={book}
            isSelected={selectedBookId === book.id}
            viewMode={viewMode}
            onSelectBook={onSelectBook}
          />
        ))}
      </main>
    )
  }

  return (
    <main className="library-scroll grid h-full min-w-0 grid-cols-1 gap-4 overflow-x-hidden overflow-y-auto pb-2 md:grid-cols-2 xl:grid-cols-3">
      {books.map((book) => (
        <LibraryBookItem
          key={book.id}
          book={book}
          isSelected={selectedBookId === book.id}
          viewMode={viewMode}
          onSelectBook={onSelectBook}
        />
      ))}
    </main>
  )
}
