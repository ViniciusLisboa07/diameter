import { getCurrentWindow } from '@tauri-apps/api/window'
import { LoaderCircle } from 'lucide-react'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { AppSidebar } from '@/components/layout/app-sidebar'
import { BookDetailsPanel } from '@/components/layout/book-details-panel'
import { LibraryCanvas } from '@/components/layout/library-canvas'
import { TopBar, type LibraryViewMode } from '@/components/layout/top-bar'
import { EpubReaderScreen } from '@/components/reader/epub-reader-screen'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { useTheme } from '@/hooks/use-theme'
import {
  deleteBook,
  importBooks,
  listBooks,
  readEpub,
  saveReadingProgress,
  type EpubReadResult,
  type ImportBooksResult,
  updateBookMetadata,
} from '@/lib/books-repository'
import { cn } from '@/lib/utils'
import type { Book } from '@/types/book'

type ReaderOpenTiming = {
  bookId: number
  clickStartedAt: number
  invokeStartedAt: number
  invokeFinishedAt: number
}

type PendingReaderOpen = {
  bookId: number
  clickStartedAt: number
  requestId: number
}

type MetadataDraft = {
  title: string
  author: string
  description: string
  tags: string[]
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function summarizeImportResult(result: ImportBooksResult): string {
  if (result.importedCount === 0 && result.rejected.length === 0) {
    return 'Nenhum arquivo foi processado.'
  }

  if (result.rejected.length === 0) {
    return `${result.importedCount} livro(s) importado(s) com sucesso.`
  }

  return `${result.importedCount} importado(s), ${result.rejected.length} rejeitado(s).`
}

function waitForReaderLoadingPaint(): Promise<void> {
  return new Promise((resolve) => {
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        window.setTimeout(resolve, 120)
      })
    })
  })
}

export function AppShell() {
  const { theme, toggleTheme } = useTheme()
  const [books, setBooks] = useState<Book[]>([])
  const [selectedBookId, setSelectedBookId] = useState<number | null>(null)
  const [viewMode, setViewMode] = useState<LibraryViewMode>('grid')
  const [searchQuery, setSearchQuery] = useState('')
  const [activeTag, setActiveTag] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [isDragActive, setIsDragActive] = useState(false)
  const [isImporting, setIsImporting] = useState(false)
  const [importFeedback, setImportFeedback] = useState<string | null>(null)
  const [isOpeningReader, setIsOpeningReader] = useState(false)
  const [isDeletingBook, setIsDeletingBook] = useState(false)
  const [readerError, setReaderError] = useState<string | null>(null)
  const [activeReader, setActiveReader] = useState<EpubReadResult | null>(null)
  const [readerOpenTiming, setReaderOpenTiming] = useState<ReaderOpenTiming | null>(null)
  const [pendingReaderOpen, setPendingReaderOpen] = useState<PendingReaderOpen | null>(null)
  const importInFlightRef = useRef(false)
  const readerOpenRequestIdRef = useRef(0)
  const startedReaderOpenRequestIdsRef = useRef(new Set<number>())

  const loadLibrary = useCallback(async () => {
    setIsLoading(true)
    setLoadError(null)

    try {
      const dbBooks = await listBooks()
      setBooks(dbBooks)

      if (dbBooks.length > 0) {
        setSelectedBookId((current) => {
          if (current && dbBooks.some((book) => book.id === current)) {
            return current
          }

          return dbBooks[0].id
        })
      } else {
        setSelectedBookId(null)
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Falha ao carregar livros do banco local.'
      setLoadError(message)
    } finally {
      setIsLoading(false)
    }
  }, [])

  const handleSaveMetadata = useCallback(async (bookId: number, draft: MetadataDraft) => {
    await updateBookMetadata({ bookId, ...draft })

    setBooks((currentBooks) =>
      currentBooks.map((currentBook) => {
        if (currentBook.id !== bookId) {
          return currentBook
        }

        return {
          ...currentBook,
          title: draft.title,
          author: draft.author,
          description: draft.description,
          tags: draft.tags,
        }
      }),
    )
  }, [])

  const handleOpenReader = useCallback(async (bookId: number) => {
    const clickStartedAt = performance.now()
    const requestId = readerOpenRequestIdRef.current + 1
    readerOpenRequestIdRef.current = requestId
    console.groupCollapsed(`[reader/open] book ${bookId}`)
    console.info('[reader/open] clique em "Ler" recebido', { bookId })
    setReaderError(null)
    setReaderOpenTiming(null)
    setIsOpeningReader(true)
    setPendingReaderOpen({ bookId, clickStartedAt, requestId })
  }, [])

  useEffect(() => {
    if (!pendingReaderOpen || startedReaderOpenRequestIdsRef.current.has(pendingReaderOpen.requestId)) {
      return
    }

    startedReaderOpenRequestIdsRef.current.add(pendingReaderOpen.requestId)

    async function openReader(request: PendingReaderOpen) {
      await waitForReaderLoadingPaint()

      try {
        const invokeStartedAt = performance.now()
        const content = await readEpub(request.bookId)
        const invokeFinishedAt = performance.now()

        if (readerOpenRequestIdRef.current !== request.requestId) {
          return
        }

        setReaderOpenTiming({
          bookId: request.bookId,
          clickStartedAt: request.clickStartedAt,
          invokeStartedAt,
          invokeFinishedAt,
        })
        console.info('[reader/open] Tauri read_epub concluido', {
          bookId: request.bookId,
          chapters: content.chapters.length,
          invokeMs: Math.round(invokeFinishedAt - invokeStartedAt),
          clickToInvokeEndMs: Math.round(invokeFinishedAt - request.clickStartedAt),
        })
        setActiveReader(content)
      } catch (error) {
        if (readerOpenRequestIdRef.current !== request.requestId) {
          return
        }

        const message = error instanceof Error ? error.message : 'Falha ao abrir EPUB.'
        console.error('[reader/open] falha ao abrir EPUB', {
          bookId: request.bookId,
          elapsedMs: Math.round(performance.now() - request.clickStartedAt),
          error,
        })
        console.groupEnd()
        setReaderError(message)
      } finally {
        if (readerOpenRequestIdRef.current === request.requestId) {
          setPendingReaderOpen(null)
          setIsOpeningReader(false)
        }
      }
    }

    void openReader(pendingReaderOpen)
  }, [pendingReaderOpen])

  const handleDeleteBook = useCallback(async (bookId: number) => {
    const confirmed = window.confirm('Tem certeza que deseja excluir este livro? Esta ação não pode ser desfeita.')
    if (!confirmed) {
      return
    }

    setIsDeletingBook(true)
    setReaderError(null)

    try {
      await deleteBook(bookId)

      setBooks((currentBooks) => currentBooks.filter((book) => book.id !== bookId))
      setActiveReader((currentReader) => (currentReader?.bookId === bookId ? null : currentReader))
      setImportFeedback('Livro excluído com sucesso.')
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Falha ao excluir livro.'
      setReaderError(message)
    } finally {
      setIsDeletingBook(false)
    }
  }, [])

  const handleProgressChange = useCallback(
    async (chapterIndex: number, progressPercent: number) => {
      if (!activeReader) {
        return
      }

      const readerBookId = activeReader.bookId
      const normalizedProgress = Math.max(0, Math.min(100, progressPercent))
      const lastPosition = `chapter_index:${chapterIndex}`

      const progressStartedAt = performance.now()
      await saveReadingProgress(readerBookId, lastPosition, normalizedProgress)
      console.info('[reader/open] progresso de leitura salvo', {
        bookId: readerBookId,
        chapterIndex,
        progressPercent: normalizedProgress,
        elapsedMs: Math.round(performance.now() - progressStartedAt),
      })

      setBooks((currentBooks) => {
        let didChange = false
        const nextBooks = currentBooks.map((book) => {
          if (book.id !== readerBookId) {
            return book
          }

          if (book.progress === normalizedProgress) {
            return book
          }

          didChange = true
          return {
            ...book,
            progress: normalizedProgress,
          }
        })

        return didChange ? nextBooks : currentBooks
      })
    },
    [activeReader],
  )

  const handleCloseReader = useCallback(() => {
    setReaderOpenTiming(null)
    setActiveReader(null)
    void loadLibrary()
  }, [loadLibrary])

  useEffect(() => {
    void loadLibrary()
  }, [loadLibrary])

  useEffect(() => {
    if (!isTauriRuntime()) {
      return
    }

    let isDisposed = false
    let unlisten: (() => void) | null = null

    const registerListener = async () => {
      const detach = await getCurrentWindow().onDragDropEvent(async (event) => {
        if (event.payload.type === 'enter' || event.payload.type === 'over') {
          setIsDragActive(true)
          return
        }

        if (event.payload.type === 'leave') {
          setIsDragActive(false)
          return
        }

        if (event.payload.type === 'drop') {
          if (importInFlightRef.current) {
            return
          }

          importInFlightRef.current = true
          setIsDragActive(false)
          setImportFeedback(null)
          setIsImporting(true)

          try {
            const result = await importBooks(event.payload.paths)
            setImportFeedback(summarizeImportResult(result))
            await loadLibrary()
          } catch (error) {
            const message = error instanceof Error ? error.message : 'Falha ao importar arquivos.'
            setImportFeedback(`Erro na importação: ${message}`)
          } finally {
            importInFlightRef.current = false
            setIsImporting(false)
          }
        }
      })

      if (isDisposed) {
        detach()
        return
      }

      unlisten = detach
    }

    void registerListener()

    return () => {
      isDisposed = true
      unlisten?.()
    }
  }, [loadLibrary])

  const allTags = useMemo(() => {
    return [...new Set(books.flatMap((book) => book.tags))].sort((left, right) => left.localeCompare(right))
  }, [books])

  const normalizedQuery = searchQuery.trim().toLowerCase()

  const filteredBooks = useMemo(() => {
    return books.filter((book) => {
      const matchesSearch =
        normalizedQuery.length === 0 ||
        book.title.toLowerCase().includes(normalizedQuery) ||
        book.author.toLowerCase().includes(normalizedQuery)

      const matchesTag = !activeTag || book.tags.includes(activeTag)

      return matchesSearch && matchesTag
    })
  }, [books, normalizedQuery, activeTag])

  useEffect(() => {
    if (filteredBooks.length === 0) {
      setSelectedBookId(null)
      return
    }

    setSelectedBookId((current) => {
      if (current && filteredBooks.some((book) => book.id === current)) {
        return current
      }

      return filteredBooks[0].id
    })
  }, [filteredBooks])

  const selectedBook = useMemo(() => {
    if (filteredBooks.length === 0 || !selectedBookId) {
      return null
    }

    return filteredBooks.find((book) => book.id === selectedBookId) ?? filteredBooks[0]
  }, [filteredBooks, selectedBookId])

  if (activeReader) {
    return (
      <EpubReaderScreen
        bookId={activeReader.bookId}
        bookTitle={activeReader.bookTitle}
        chapters={activeReader.chapters}
        initialChapterIndex={activeReader.lastChapterIndex}
        openTiming={readerOpenTiming?.bookId === activeReader.bookId ? readerOpenTiming : null}
        onBack={handleCloseReader}
        onProgressChange={handleProgressChange}
      />
    )
  }

  return (
    <div className="relative grid h-screen w-full max-w-full grid-cols-1 gap-4 overflow-hidden p-4 lg:grid-cols-[260px_minmax(0,1fr)_340px] lg:p-5">
      <AppSidebar />

      <section className="relative flex min-h-0 flex-col gap-4 overflow-hidden">
        <TopBar
          theme={theme}
          onToggleTheme={toggleTheme}
          viewMode={viewMode}
          searchQuery={searchQuery}
          onSearchQueryChange={setSearchQuery}
          onClearSearch={() => setSearchQuery('')}
          onChangeViewMode={setViewMode}
        />

        <div className="flex flex-wrap items-center gap-2">
          <Badge variant="secondary">Importação por drag and drop</Badge>
          {isImporting && <p className="text-xs text-muted-foreground">Importando arquivos...</p>}
          {!isImporting && importFeedback && <p className="text-xs text-muted-foreground">{importFeedback}</p>}
          {readerError && <p className="text-xs text-red-500">{readerError}</p>}
        </div>

        {books.length > 0 && (
          <div className="rounded-xl border bg-card/70 p-3">
            <div className="mb-2 flex flex-wrap items-center gap-2">
              <p className="text-xs text-muted-foreground">Filtrar por tags:</p>
              {allTags.map((tag) => (
                <Button
                  key={tag}
                  size="sm"
                  variant={activeTag === tag ? 'default' : 'outline'}
                  className="h-7 rounded-full px-3 text-xs"
                  onClick={() => setActiveTag((current) => (current === tag ? null : tag))}
                >
                  {tag}
                </Button>
              ))}
            </div>

            <div className="flex flex-wrap items-center gap-2">
              {normalizedQuery && <Badge variant="outline">Busca ativa: {searchQuery.trim()}</Badge>}
              {activeTag && <Badge>Tag ativa: {activeTag}</Badge>}
              {(normalizedQuery || activeTag) && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 px-2 text-xs"
                  onClick={() => {
                    setSearchQuery('')
                    setActiveTag(null)
                  }}
                >
                  Limpar filtros
                </Button>
              )}
            </div>
          </div>
        )}

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
              Biblioteca vazia. Arraste EPUB/PDF para começar.
            </CardContent>
          </Card>
        ) : filteredBooks.length === 0 ? (
          <Card className="h-full border bg-card/85">
            <CardContent className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
              Nenhum livro encontrado para os filtros atuais.
            </CardContent>
          </Card>
        ) : (
          <LibraryCanvas
            books={filteredBooks}
            selectedBookId={selectedBook?.id ?? filteredBooks[0].id}
            viewMode={viewMode}
            onSelectBook={setSelectedBookId}
          />
        )}

        <div
          className={cn(
            'pointer-events-none absolute inset-0 flex items-center justify-center rounded-xl border-2 border-dashed border-primary/60 bg-primary/10 opacity-0 transition-opacity',
            isDragActive && 'opacity-100',
          )}
        >
          <div className="rounded-lg bg-card px-6 py-4 text-center shadow-soft">
            <p className="font-semibold text-foreground">Solte seus ebooks aqui</p>
            <p className="text-sm text-muted-foreground">Formatos aceitos: EPUB e PDF</p>
          </div>
        </div>
      </section>

      {selectedBook ? (
        <BookDetailsPanel
          book={selectedBook}
          onSaveMetadata={handleSaveMetadata}
          onReadBook={handleOpenReader}
          onDeleteBook={handleDeleteBook}
          isOpeningReader={isOpeningReader}
          isDeletingBook={isDeletingBook}
        />
      ) : (
        <Card className="h-full border bg-card/85">
          <CardContent className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
            Selecione um livro para ver detalhes.
          </CardContent>
        </Card>
      )}

      {isOpeningReader && selectedBook ? (
        <div
          className="absolute inset-0 z-50 flex items-center justify-center bg-background/72 px-4 backdrop-blur-sm"
          role="status"
          aria-live="polite"
        >
          <div className="flex w-full max-w-sm flex-col items-center gap-4 rounded-lg border bg-card px-6 py-7 text-center shadow-soft">
            <LoaderCircle className="h-7 w-7 animate-spin text-primary" />
            <div className="space-y-1">
              <p className="text-sm font-semibold text-foreground">Preparando leitura</p>
              <p className="line-clamp-2 text-sm text-muted-foreground">{selectedBook.title}</p>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  )
}
