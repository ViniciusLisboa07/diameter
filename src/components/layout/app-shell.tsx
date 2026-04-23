import { useEffect, useMemo, useState } from 'react'

import { AppSidebar } from '@/components/layout/app-sidebar'
import { BookDetailsPanel } from '@/components/layout/book-details-panel'
import { LibraryCanvas } from '@/components/layout/library-canvas'
import { TopBar, type LibraryViewMode } from '@/components/layout/top-bar'
import { Card, CardContent } from '@/components/ui/card'
import { useTheme } from '@/hooks/use-theme'
import { listBooks } from '@/lib/books-repository'
import type { Book } from '@/types/book'

export function AppShell() {
  const { theme, toggleTheme } = useTheme()
  const [books, setBooks] = useState<Book[]>([])
  const [selectedBookId, setSelectedBookId] = useState<number | null>(null)
  const [viewMode, setViewMode] = useState<LibraryViewMode>('grid')
  const [isLoading, setIsLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)

  useEffect(() => {
    let isMounted = true

    const loadLibrary = async () => {
      setIsLoading(true)
      setLoadError(null)

      try {
        const dbBooks = await listBooks()

        if (!isMounted) {
          return
        }

        setBooks(dbBooks)

        if (dbBooks.length > 0) {
          setSelectedBookId((current) => current ?? dbBooks[0].id)
        }
      } catch (error) {
        if (!isMounted) {
          return
        }

        const message = error instanceof Error ? error.message : 'Falha ao carregar livros do banco local.'
        setLoadError(message)
      } finally {
        if (isMounted) {
          setIsLoading(false)
        }
      }
    }

    void loadLibrary()

    return () => {
      isMounted = false
    }
  }, [])

  const selectedBook = useMemo(() => {
    if (books.length === 0) {
      return null
    }

    return books.find((book) => book.id === selectedBookId) ?? books[0]
  }, [books, selectedBookId])

  return (
    <div className="grid min-h-screen grid-cols-1 gap-4 p-4 lg:grid-cols-[260px_1fr_340px] lg:p-5">
      <AppSidebar />

      <section className="flex min-h-0 flex-col gap-4">
        <TopBar
          theme={theme}
          onToggleTheme={toggleTheme}
          viewMode={viewMode}
          onChangeViewMode={setViewMode}
        />

        {isLoading ? (
          <Card className="h-full border bg-card/85">
            <CardContent className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
              Carregando biblioteca local...
            </CardContent>
          </Card>
        ) : loadError ? (
          <Card className="h-full border bg-card/85">
            <CardContent className="flex h-full flex-col items-center justify-center gap-2 p-6 text-center">
              <p className="text-sm font-medium text-foreground">Não foi possível carregar os dados locais.</p>
              <p className="text-xs text-muted-foreground">{loadError}</p>
            </CardContent>
          </Card>
        ) : books.length === 0 ? (
          <Card className="h-full border bg-card/85">
            <CardContent className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
              Biblioteca vazia.
            </CardContent>
          </Card>
        ) : (
          <LibraryCanvas
            books={books}
            selectedBookId={selectedBook?.id ?? books[0].id}
            viewMode={viewMode}
            onSelectBook={setSelectedBookId}
          />
        )}
      </section>

      {selectedBook ? (
        <BookDetailsPanel book={selectedBook} />
      ) : (
        <Card className="h-full border bg-card/85">
          <CardContent className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
            Selecione um livro para ver detalhes.
          </CardContent>
        </Card>
      )}
    </div>
  )
}
