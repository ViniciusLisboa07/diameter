import { ArrowLeft, ChevronLeft, ChevronRight, ListTree, Palette } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'

import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

type ReaderTheme = 'paper' | 'night'

type ReaderChapter = {
  title: string
  content: string
  html?: string
}

type EpubReaderScreenProps = {
  bookId: number
  bookTitle: string
  chapters: ReaderChapter[]
  initialChapterIndex: number
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
  paper: 'border-[#dccaaa]/70 bg-[#fffaf0]/70 shadow-[0_24px_80px_rgba(89,68,39,0.12)]',
  night: 'border-white/10 bg-[#182131]/72 shadow-[0_24px_80px_rgba(0,0,0,0.28)]',
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
  paper: 'border-[#d8c3a1]/80 bg-[#fffaf0]/78 text-[#3b2b19] shadow-[0_16px_45px_rgba(89,68,39,0.16)] hover:bg-[#fffaf0]',
  night: 'border-white/10 bg-[#1a2434]/78 text-[#f2e6d2] shadow-[0_16px_45px_rgba(0,0,0,0.32)] hover:bg-[#243149]',
}

const readerSidebarStyles: Record<ReaderTheme, string> = {
  paper: 'border-[#d9c6a5]/80 bg-[#fffaf0]/82 shadow-[18px_0_55px_rgba(89,68,39,0.12)]',
  night: 'border-white/10 bg-[#172132]/86 shadow-[18px_0_55px_rgba(0,0,0,0.26)]',
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

function computeProgress(chapterIndex: number, chapterCount: number): number {
  if (chapterCount <= 1) {
    return chapterIndex > 0 ? 100 : 0
  }

  return Math.round((chapterIndex / (chapterCount - 1)) * 100)
}

export function EpubReaderScreen({
  bookId,
  bookTitle,
  chapters,
  initialChapterIndex,
  onBack,
  onProgressChange,
}: EpubReaderScreenProps) {
  const [readerTheme, setReaderTheme] = useState<ReaderTheme>('paper')
  const [isChapterListOpen, setIsChapterListOpen] = useState(false)
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

  const goToChapter = useCallback((chapterIndex: number) => {
    setCurrentChapterIndex(Math.max(0, Math.min(chapterIndex, chapters.length - 1)))
    window.scrollTo({ top: 0, behavior: 'smooth' })
  }, [chapters.length])

  const goToPreviousChapter = useCallback(() => {
    setCurrentChapterIndex((current) => {
      const nextChapterIndex = Math.max(0, current - 1)
      if (nextChapterIndex !== current) {
        window.scrollTo({ top: 0, behavior: 'smooth' })
      }
      return nextChapterIndex
    })
  }, [])

  const goToNextChapter = useCallback(() => {
    setCurrentChapterIndex((current) => {
      const nextChapterIndex = Math.min(chapters.length - 1, current + 1)
      if (nextChapterIndex !== current) {
        window.scrollTo({ top: 0, behavior: 'smooth' })
      }
      return nextChapterIndex
    })
  }, [chapters.length])

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

  const chapterList = (
    <nav aria-label="Capítulos e seções do livro" className="space-y-1.5">
      {chapters.map((chapter, chapterIndex) => {
        const isCurrentChapter = chapterIndex === currentChapterIndex

        return (
          <button
            key={`${chapter.title}-${chapterIndex}`}
            type="button"
            aria-current={isCurrentChapter ? 'page' : undefined}
            onClick={() => {
              goToChapter(chapterIndex)
              setIsChapterListOpen(false)
            }}
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
          className="fixed inset-0 z-30 bg-black/28 backdrop-blur-[2px] lg:hidden"
          onClick={() => setIsChapterListOpen(false)}
        />
      ) : null}

      <aside
        id="reader-chapter-sidebar"
        className={cn(
          'fixed inset-y-0 left-0 z-40 flex w-[19rem] max-w-[calc(100vw-2rem)] flex-col border-r px-4 py-5 backdrop-blur-2xl transition-transform duration-200 lg:translate-x-0',
          isChapterListOpen ? 'translate-x-0' : '-translate-x-full',
          readerSidebarStyles[readerTheme],
        )}
      >
        <div className="mb-5 flex items-start justify-between gap-3">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.24em] opacity-55">Sumário</p>
            <h2 className="mt-2 line-clamp-2 font-display text-2xl leading-tight">{bookTitle}</h2>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="Fechar sumário"
            className="lg:hidden"
            onClick={() => setIsChapterListOpen(false)}
          >
            <ChevronLeft className="h-5 w-5" />
          </Button>
        </div>

        <p className="mb-3 text-sm opacity-70">
          {chapters.length} {chapters.length === 1 ? 'seção' : 'seções'}
        </p>

        <div className="min-h-0 flex-1 overflow-y-auto pr-1">{chapterList}</div>
      </aside>

      <nav
        aria-label="Navegação persistente entre capítulos"
        className="pointer-events-none fixed inset-x-0 inset-y-0 z-20 hidden items-center justify-between px-3 lg:flex lg:pl-[20rem] xl:px-6"
      >
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label="Ir para o capítulo anterior"
          title="Capítulo anterior (seta para esquerda)"
          onClick={goToPreviousChapter}
          disabled={!canGoToPreviousChapter}
          className={cn(
            'reader-floating-nav pointer-events-auto h-14 w-14 rounded-full border backdrop-blur-xl transition-all duration-200',
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
            'reader-floating-nav pointer-events-auto h-14 w-14 rounded-full border backdrop-blur-xl transition-all duration-200',
            readerFloatingNavStyles[readerTheme],
          )}
        >
          <ChevronRight className="h-6 w-6" />
        </Button>
      </nav>

      <header className={cn('sticky top-0 z-10 border-b backdrop-blur-xl lg:pl-[19rem]', readerHeaderStyles[readerTheme])}>
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
              onClick={() => setIsChapterListOpen((current) => !current)}
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
              onClick={() => setReaderTheme('paper')}
            >
              Papel
            </Button>
            <Button
              size="sm"
              variant={readerTheme === 'night' ? 'default' : 'outline'}
              onClick={() => setReaderTheme('night')}
            >
              Noturno
            </Button>
          </div>
        </div>
      </header>

      <main className="mx-auto flex w-full max-w-5xl flex-1 flex-col gap-8 px-4 py-8 sm:px-6 lg:ml-[19rem] lg:mr-0 lg:max-w-[calc(100vw-19rem)] lg:py-12">
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
            Capítulo {currentChapterIndex + 1} de {chapters.length} · {progressPercent}%
          </p>
        </div>

        <article
          className={cn(
            'mx-auto w-full max-w-3xl rounded-[2rem] border px-5 py-8 backdrop-blur sm:px-10 sm:py-12 lg:px-14',
            readerSurfaceStyles[readerTheme],
          )}
        >
          <h2 className="mb-8 font-display text-3xl leading-[1.08] tracking-[-0.02em] sm:text-4xl">
            {currentChapter.title}
          </h2>
          {currentChapter.html ? (
            <div
              className={cn(
                'epub-content',
                readerTheme === 'night' ? 'epub-content-night' : 'epub-content-paper',
              )}
              dangerouslySetInnerHTML={{ __html: currentChapter.html }}
            />
          ) : (
            <div
              className={cn(
                'epub-content epub-content-plain',
                readerTheme === 'night' ? 'epub-content-night' : 'epub-content-paper',
              )}
            >
              {currentChapter.content
                .split('\n\n')
                .map((paragraph) => paragraph.trim())
                .filter(Boolean)
                .map((paragraph, paragraphIndex) => (
                  <p key={`${currentChapter.title}-${paragraphIndex}`}>{paragraph}</p>
                ))}
            </div>
          )}
        </article>

        <div className="mx-auto flex w-full max-w-3xl items-center justify-between border-t border-current/10 pt-5">
          <Button
            variant="outline"
            aria-label="Ir para o capítulo anterior"
            onClick={goToPreviousChapter}
            disabled={!canGoToPreviousChapter}
          >
            <ChevronLeft className="h-4 w-4" />
            Anterior
          </Button>

          <Button
            variant="outline"
            aria-label="Ir para o próximo capítulo"
            onClick={goToNextChapter}
            disabled={!canGoToNextChapter}
          >
            Próximo
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </main>
    </section>
  )
}
