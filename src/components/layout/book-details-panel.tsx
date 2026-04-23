import type { Book } from '@/types/book'

import { BookCover } from '@/components/layout/book-cover'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Separator } from '@/components/ui/separator'

type BookDetailsPanelProps = {
  book: Book
}

export function BookDetailsPanel({ book }: BookDetailsPanelProps) {
  return (
    <aside className="h-full">
      <Card className="h-full border bg-card/85">
        <CardHeader>
          <div className="mb-3 max-w-[180px]">
            <BookCover book={book} />
          </div>
          <CardTitle className="font-display text-2xl leading-tight">{book.title}</CardTitle>
          <CardDescription>{book.author}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <Badge>{book.format}</Badge>
            {book.tags.map((tag) => (
              <Badge key={tag} variant="secondary">
                {tag}
              </Badge>
            ))}
          </div>

          <Separator />

          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">Progresso de leitura</p>
            <div className="h-2 rounded-full bg-secondary">
              <div className="h-full rounded-full bg-primary" style={{ width: `${book.progress}%` }} />
            </div>
            <p className="text-xs text-muted-foreground">{book.progress}% concluído</p>
          </div>

          <Separator />

          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">Resumo</p>
            <p className="text-sm leading-relaxed text-foreground/90">{book.description}</p>
          </div>
        </CardContent>
      </Card>
    </aside>
  )
}
