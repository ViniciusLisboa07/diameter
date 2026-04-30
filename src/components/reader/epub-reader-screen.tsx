import { ArrowLeft, ChevronLeft, ChevronRight, ListTree, Palette, PanelLeftClose, PanelLeftOpen } from 'lucide-react'
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

type ReaderTheme = 'paper' | 'night'

type ReaderChapter = {
  title: string
  html: string
}

type EpubReaderScreenProps = {
  bookId: number
  bookTitle: string
  chapters: ReaderChapter[]
  initialChapterIndex: number
  openTiming?: {
    bookId: number
    clickStartedAt: number
    invokeStartedAt: number
    invokeFinishedAt: number
  } | null
  onBack: () => void
  onProgressChange: (chapterIndex: number, progressPercent: number) => Promise<void>
}

const readerThemeStyles: Record<ReaderTheme, string> = {
  paper: 'bg-[#f6efe1] text-[#2f271a]',
  night: 'bg-[#0f1724] text-[#e8e1d4]',
}

const readerHeaderStyles: Record<ReaderTheme, string> = {
  paper: 'border-[#d7c6a8]/70 bg-[#f6efe1]/90',
  night: 'border-white/10 bg-[#0f1724]/90',
}

const readerSurfaceStyles: Record<ReaderTheme, string> = {
  paper: 'border-[#dccaaa]/70 bg-[#fffaf0] shadow-[0_12px_34px_rgba(89,68,39,0.10)]',
  night: 'border-white/10 bg-[#182131] shadow-[0_14px_38px_rgba(0,0,0,0.22)]',
}

const readerProgressTrackStyles: Record<ReaderTheme, string> = {
  paper: 'bg-[#d8c7a8]/70',
  night: 'bg-white/10',
}

const readerProgressFillStyles: Record<ReaderTheme, string> = {
  paper: 'bg-[#8d5f32]',
  night: 'bg-[#d9b26f]',
}

const readerFloatingNavStyles: Record<ReaderTheme, string> = {
  paper: 'border-[#d8c3a1]/80 bg-[#fffaf0] text-[#3b2b19] shadow-[0_10px_26px_rgba(89,68,39,0.12)] hover:bg-[#fffaf0]',
  night: 'border-white/10 bg-[#1a2434] text-[#f2e6d2] shadow-[0_10px_28px_rgba(0,0,0,0.24)] hover:bg-[#243149]',
}

const readerSidebarStyles: Record<ReaderTheme, string> = {
  paper: 'border-[#d9c6a5]/80 bg-[#fffaf0] shadow-[10px_0_28px_rgba(89,68,39,0.10)]',
  night: 'border-white/10 bg-[#172132] shadow-[10px_0_30px_rgba(0,0,0,0.22)]',
}

const readerSidebarItemStyles: Record<ReaderTheme, { active: string; inactive: string }> = {
  paper: {
    active: 'border-[#8d5f32]/45 bg-[#8d5f32]/12 text-[#2f2114]',
    inactive: 'border-transparent text-[#5a4630] hover:border-[#d8c3a1] hover:bg-[#8d5f32]/8 hover:text-[#2f2114]',
  },
  night: {
    active: 'border-[#d9b26f]/45 bg-[#d9b26f]/14 text-[#fff3dc]',
    inactive: 'border-transparent text-[#cfc3b2] hover:border-white/10 hover:bg-white/7 hover:text-[#fff3dc]',
  },
}

const loggedInitialRenderKeys = new Set<string>()

function computeProgress(chapterIndex: number, chapterCount: number): number {
  if (chapterCount <= 1) {
    return chapterIndex > 0 ? 100 : 0
  }

  return Math.round((chapterIndex / (chapterCount - 1)) * 100)
}

type ReaderChapterListProps = {
  chapters: ReaderChapter[]
  currentChapterIndex: number
  readerTheme: ReaderTheme
  onSelectChapter: (chapterIndex: number) => void
}

const ReaderChapterList = memo(function ReaderChapterList({
  chapters,
  currentChapterIndex,
  readerTheme,
  onSelectChapter,
}: ReaderChapterListProps) {
  return (
    <nav aria-label="Capítulos e seções do livro" className="space-y-1.5">
      {chapters.map((chapter, chapterIndex) => {
        const isCurrentChapter = chapterIndex === currentChapterIndex

        return (
          <button
            key={`${chapter.title}-${chapterIndex}`}
            type="button"
            aria-current={isCurrentChapter ? 'page' : undefined}
            onClick={() => onSelectChapter(chapterIndex)}
            className={cn(
              'w-full rounded-2xl border px-3 py-2.5 text-left text-sm leading-snug transition-colors',
              isCurrentChapter
                ? readerSidebarItemStyles[readerTheme].active
                : readerSidebarItemStyles[readerTheme].inactive,
            )}
          >
            <span className="mb-1 block text-[0.68rem] font-semibold uppercase tracking-[0.18em] opacity-55">
              {chapterIndex + 1}
            </span>
            <span className="line-clamp-2 font-medium">{chapter.title}</span>
          </button>
        )
      })}
    </nav>
  )
})

type ReaderProgressSummaryProps = {
  bookTitle: string
  chapterCount: number
  currentChapterIndex: number
  progressPercent: number
  readerTheme: ReaderTheme
}

const ReaderProgressSummary = memo(function ReaderProgressSummary({
  bookTitle,
  chapterCount,
  currentChapterIndex,
  progressPercent,
  readerTheme,
}: ReaderProgressSummaryProps) {
  return (
    <div className="mx-auto w-full max-w-3xl space-y-4">
      <p className="text-xs font-semibold uppercase tracking-[0.28em] opacity-55">Leitura atual</p>
      <h1 className="font-display text-4xl leading-[1.04] tracking-[-0.03em] sm:text-5xl">{bookTitle}</h1>
      <div className={cn('h-1.5 w-full overflow-hidden rounded-full', readerProgressTrackStyles[readerTheme])}>
        <div
          className={cn('h-full rounded-full transition-[width] duration-500', readerProgressFillStyles[readerTheme])}
          style={{ width: `${progressPercent}%` }}
        />
      </div>
      <p className="text-sm opacity-80">
        Capítulo {currentChapterIndex + 1} de {chapterCount} · {progressPercent}%
      </p>
    </div>
  )
})

type ReaderArticleProps = {
  chapter: ReaderChapter
  readerTheme: ReaderTheme
}

const ReaderArticle = memo(function ReaderArticle({ chapter, readerTheme }: ReaderArticleProps) {
  return (
    <article
      className={cn(
        'mx-auto w-full max-w-3xl rounded-[2rem] border px-5 py-8 sm:px-10 sm:py-12 lg:px-14',
        readerSurfaceStyles[readerTheme],
      )}
    >
      <h2 className="mb-8 font-display text-3xl leading-[1.08] tracking-[-0.02em] sm:text-4xl">{chapter.title}</h2>
      <div
        className={cn('epub-content', readerTheme === 'night' ? 'epub-content-night' : 'epub-content-paper')}
        dangerouslySetInnerHTML={{ __html: chapter.html }}
      />
    </article>
  )
})

type ReaderBottomNavigationProps = {
  canGoToPreviousChapter: boolean
  canGoToNextChapter: boolean
  onPreviousChapter: () => void
  onNextChapter: () => void
}

const ReaderBottomNavigation = memo(function ReaderBottomNavigation({
  canGoToPreviousChapter,
  canGoToNextChapter,
  onPreviousChapter,
  onNextChapter,
}: ReaderBottomNavigationProps) {
  return (
    <div className="mx-auto flex w-full max-w-3xl items-center justify-between border-t border-current/10 pt-5">
      <Button
        variant="outline"
        aria-label="Ir para o capítulo anterior"
        onClick={onPreviousChapter}
        disabled={!canGoToPreviousChapter}
      >
        <ChevronLeft className="h-4 w-4" />
        Anterior
      </Button>

      <Button
        variant="outline"
        aria-label="Ir para o próximo capítulo"
        onClick={onNextChapter}
        disabled={!canGoToNextChapter}
      >
        Próximo
        <ChevronRight className="h-4 w-4" />
      </Button>
    </div>
  )
})

function scrollReaderToTop() {
  window.scrollTo({ top: 0, behavior: 'auto' })
}

export const EpubReaderScreen = memo(function EpubReaderScreen({
  bookId,
  bookTitle,
  chapters,
  initialChapterIndex,
  openTiming,
  onBack,
  onProgressChange,
}: EpubReaderScreenProps) {
  const [readerTheme, setReaderTheme] = useState<ReaderTheme>('paper')
  const [isChapterListOpen, setIsChapterListOpen] = useState(false)
  const [isDesktopChapterSidebarCollapsed, setIsDesktopChapterSidebarCollapsed] = useState(true)
  const didLogInitialRenderRef = useRef(false)
  const [currentChapterIndex, setCurrentChapterIndex] = useState(() =>
    Math.max(0, Math.min(initialChapterIndex, chapters.length - 1)),
  )

  const currentChapter = chapters[currentChapterIndex]
  const progressPercent = useMemo(
    () => computeProgress(currentChapterIndex, chapters.length),
    [currentChapterIndex, chapters.length],
  )
  const canGoToPreviousChapter = currentChapterIndex > 0
  const canGoToNextChapter = currentChapterIndex < chapters.length - 1
  const closeChapterList = useCallback(() => setIsChapterListOpen(false), [])
  const toggleChapterList = useCallback(() => setIsChapterListOpen((current) => !current), [])
  const toggleDesktopChapterSidebar = useCallback(
    () => setIsDesktopChapterSidebarCollapsed((current) => !current),
    [],
  )
  const expandDesktopChapterSidebar = useCallback(() => setIsDesktopChapterSidebarCollapsed(false), [])
  const setPaperTheme = useCallback(() => setReaderTheme('paper'), [])
  const setNightTheme = useCallback(() => setReaderTheme('night'), [])

  useEffect(() => {
    if (didLogInitialRenderRef.current) {
      return
    }

    didLogInitialRenderRef.current = true
    const renderKey = openTiming ? `${bookId}:${openTiming.clickStartedAt}` : `${bookId}:unknown`
    const scheduledAt = performance.now()
    const frameId = window.requestAnimationFrame(() => {
      if (loggedInitialRenderKeys.has(renderKey)) {
        return
      }

      loggedInitialRenderKeys.add(renderKey)
      const renderedAt = performance.now()
      console.info('[reader/open] tela inicial do reader renderizada', {
        bookId,
        chapters: chapters.length,
        renderAfterStateMs: openTiming ? Math.round(renderedAt - openTiming.invokeFinishedAt) : null,
        clickToFirstRenderMs: openTiming ? Math.round(renderedAt - openTiming.clickStartedAt) : null,
        frameWaitMs: Math.round(renderedAt - scheduledAt),
      })
      console.groupEnd()
    })

    return () => window.cancelAnimationFrame(frameId)
  }, [bookId, chapters.length, openTiming])

  const goToChapter = useCallback((chapterIndex: number) => {
    setCurrentChapterIndex(Math.max(0, Math.min(chapterIndex, chapters.length - 1)))
    scrollReaderToTop()
  }, [chapters.length])

  const goToPreviousChapter = useCallback(() => {
    setCurrentChapterIndex((current) => {
      const nextChapterIndex = Math.max(0, current - 1)
      if (nextChapterIndex !== current) {
        scrollReaderToTop()
      }
      return nextChapterIndex
    })
  }, [])

  const goToNextChapter = useCallback(() => {
    setCurrentChapterIndex((current) => {
      const nextChapterIndex = Math.min(chapters.length - 1, current + 1)
      if (nextChapterIndex !== current) {
        scrollReaderToTop()
      }
      return nextChapterIndex
    })
  }, [chapters.length])

  const selectChapterAndCloseList = useCallback(
    (chapterIndex: number) => {
      goToChapter(chapterIndex)
      setIsChapterListOpen(false)
    },
    [goToChapter],
  )

  useEffect(() => {
    setCurrentChapterIndex(Math.max(0, Math.min(initialChapterIndex, chapters.length - 1)))
  }, [initialChapterIndex, chapters.length, bookId])

  useEffect(() => {
    if (chapters.length === 0) {
      return
    }

    void onProgressChange(currentChapterIndex, progressPercent)
  }, [currentChapterIndex, progressPercent, chapters.length, onProgressChange])

  useEffect(() => {
    function handleReaderKeyDown(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null
      const isTyping =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement ||
        target?.isContentEditable

      if (isTyping || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
        return
      }

      if (event.key === 'Escape') {
        event.preventDefault()
        if (isChapterListOpen) {
          setIsChapterListOpen(false)
        } else {
          onBack()
        }
      }

      if (event.key === 'ArrowLeft' && canGoToPreviousChapter) {
        event.preventDefault()
        goToPreviousChapter()
      }

      if (event.key === 'ArrowRight' && canGoToNextChapter) {
        event.preventDefault()
        goToNextChapter()
      }
    }

    window.addEventListener('keydown', handleReaderKeyDown)
    return () => window.removeEventListener('keydown', handleReaderKeyDown)
  }, [canGoToNextChapter, canGoToPreviousChapter, goToNextChapter, goToPreviousChapter, isChapterListOpen, onBack])

  return (
    <section
      className={cn(
        'reader-shell flex min-h-screen flex-col',
        readerThemeStyles[readerTheme],
        readerTheme === 'night' ? 'reader-shell-night' : 'reader-shell-paper',
      )}
    >
      {isChapterListOpen ? (
        <button
          type="button"
          aria-label="Fechar sumário"
          className="fixed inset-0 z-30 bg-black/28 lg:hidden"
          onClick={closeChapterList}
        />
      ) : null}

      <aside
        id="reader-chapter-sidebar"
        className={cn(
          'fixed inset-y-0 left-0 z-40 flex w-[19rem] max-w-[calc(100vw-2rem)] flex-col border-r px-4 py-5 transition-transform duration-200 lg:translate-x-0 lg:transition-[width,padding] lg:duration-200',
          isChapterListOpen ? 'translate-x-0' : '-translate-x-full',
          isDesktopChapterSidebarCollapsed ? 'lg:w-[5.25rem] lg:px-3' : 'lg:w-[19rem] lg:px-4',
          readerSidebarStyles[readerTheme],
        )}
      >
        <div
          className={cn(
            'mb-5 flex items-start justify-between gap-3',
            isDesktopChapterSidebarCollapsed ? 'lg:flex-col lg:items-center lg:gap-4' : null,
          )}
        >
          <div className={cn(isDesktopChapterSidebarCollapsed ? 'lg:hidden' : null)}>
            <p className="text-xs font-semibold uppercase tracking-[0.24em] opacity-55">Sumário</p>
            <h2 className="mt-2 line-clamp-2 font-display text-2xl leading-tight">{bookTitle}</h2>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={isDesktopChapterSidebarCollapsed ? 'Expandir sumário' : 'Recolher sumário'}
            aria-expanded={!isDesktopChapterSidebarCollapsed}
            aria-controls="reader-chapter-list"
            title={isDesktopChapterSidebarCollapsed ? 'Expandir sumário' : 'Recolher sumário'}
            className="hidden lg:inline-flex"
            onClick={toggleDesktopChapterSidebar}
          >
            {isDesktopChapterSidebarCollapsed ? (
              <PanelLeftOpen className="h-5 w-5" />
            ) : (
              <PanelLeftClose className="h-5 w-5" />
            )}
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="Fechar sumário"
            className="lg:hidden"
            onClick={closeChapterList}
          >
            <ChevronLeft className="h-5 w-5" />
          </Button>
        </div>

        <div
          className={cn(
            'hidden min-h-0 flex-1 flex-col items-center gap-4 lg:flex',
            isDesktopChapterSidebarCollapsed ? null : 'lg:hidden',
          )}
        >
          <button
            type="button"
            aria-current="page"
            aria-label={`Capítulo atual: ${currentChapter.title}`}
            onClick={expandDesktopChapterSidebar}
            className={cn(
              'flex h-12 w-12 items-center justify-center rounded-2xl border text-sm font-semibold transition-colors',
              readerSidebarItemStyles[readerTheme].active,
            )}
            title={currentChapter.title}
          >
            {currentChapterIndex + 1}
          </button>
          <div className="h-px w-8 bg-current/15" />
          <p className="origin-center rotate-180 text-center text-[0.68rem] font-semibold uppercase tracking-[0.18em] opacity-55 [writing-mode:vertical-rl]">
            Sumário
          </p>
        </div>

        <div
          id="reader-chapter-list"
          className={cn(
            'min-h-0 flex-1 overflow-y-auto pr-1',
            isDesktopChapterSidebarCollapsed ? 'lg:hidden' : null,
          )}
        >
          <p className="mb-3 text-sm opacity-70">
            {chapters.length} {chapters.length === 1 ? 'seção' : 'seções'}
          </p>
          <ReaderChapterList
            chapters={chapters}
            currentChapterIndex={currentChapterIndex}
            readerTheme={readerTheme}
            onSelectChapter={selectChapterAndCloseList}
          />
        </div>
      </aside>

      <nav
        aria-label="Navegação persistente entre capítulos"
        className={cn(
          'pointer-events-none fixed inset-y-0 z-20 hidden items-center lg:flex',
          isDesktopChapterSidebarCollapsed
            ? 'lg:left-[5.25rem] lg:w-[calc(100vw-5.25rem)]'
            : 'lg:left-[19rem] lg:w-[calc(100vw-19rem)]',
        )}
      >
        <div className="mx-auto flex w-full max-w-[78rem] items-center justify-between px-3 xl:px-0">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="Ir para o capítulo anterior"
            title="Capítulo anterior (seta para esquerda)"
            onClick={goToPreviousChapter}
            disabled={!canGoToPreviousChapter}
            className={cn(
              'reader-floating-nav pointer-events-auto h-14 w-14 rounded-full border transition-all duration-200',
              readerFloatingNavStyles[readerTheme],
            )}
          >
            <ChevronLeft className="h-6 w-6" />
          </Button>

          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="Ir para o próximo capítulo"
            title="Próximo capítulo (seta para direita)"
            onClick={goToNextChapter}
            disabled={!canGoToNextChapter}
            className={cn(
              'reader-floating-nav pointer-events-auto h-14 w-14 rounded-full border transition-all duration-200',
              readerFloatingNavStyles[readerTheme],
            )}
          >
            <ChevronRight className="h-6 w-6" />
          </Button>
        </div>
      </nav>

      <header
        className={cn(
          'sticky top-0 z-10 border-b',
          isDesktopChapterSidebarCollapsed ? 'lg:ml-[5.25rem]' : 'lg:ml-[19rem]',
          readerHeaderStyles[readerTheme],
        )}
      >
        <div className="mx-auto flex w-full max-w-5xl flex-col gap-3 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-center gap-2">
            <Button variant="ghost" onClick={onBack}>
              <ArrowLeft className="h-4 w-4" />
              Voltar para biblioteca
            </Button>

            <Button
              type="button"
              variant="outline"
              aria-expanded={isChapterListOpen}
              aria-controls="reader-chapter-sidebar"
              onClick={toggleChapterList}
              className="lg:hidden"
            >
              <ListTree className="h-4 w-4" />
              Sumário
            </Button>
          </div>

          <div className="flex items-center gap-2">
            <Palette className="h-4 w-4" />
            <Button
              size="sm"
              variant={readerTheme === 'paper' ? 'default' : 'outline'}
              onClick={setPaperTheme}
            >
              Papel
            </Button>
            <Button
              size="sm"
              variant={readerTheme === 'night' ? 'default' : 'outline'}
              onClick={setNightTheme}
            >
              Noturno
            </Button>
          </div>
        </div>
      </header>

      <main
        className={cn(
          'mx-auto flex w-full max-w-5xl flex-1 flex-col gap-8 px-4 py-8 sm:px-6 lg:mr-0 lg:py-12',
          isDesktopChapterSidebarCollapsed
            ? 'lg:ml-[5.25rem] lg:max-w-[calc(100vw-5.25rem)]'
            : 'lg:ml-[19rem] lg:max-w-[calc(100vw-19rem)]',
        )}
      >
        <ReaderProgressSummary
          bookTitle={bookTitle}
          chapterCount={chapters.length}
          currentChapterIndex={currentChapterIndex}
          progressPercent={progressPercent}
          readerTheme={readerTheme}
        />

        <ReaderArticle chapter={currentChapter} readerTheme={readerTheme} />

        <ReaderBottomNavigation
          canGoToPreviousChapter={canGoToPreviousChapter}
          canGoToNextChapter={canGoToNextChapter}
          onPreviousChapter={goToPreviousChapter}
          onNextChapter={goToNextChapter}
        />
      </main>
    </section>
  )
})
