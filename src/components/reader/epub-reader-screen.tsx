import { ArrowLeft, ChevronLeft, ChevronRight, Palette } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'

import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

type ReaderTheme = 'paper' | 'night'

type ReaderChapter = {
  title: string
  content: string
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
  paper: 'bg-[#f7f2e8] text-[#2f271a]',
  night: 'bg-[#111827] text-[#e5e7eb]',
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
  const [currentChapterIndex, setCurrentChapterIndex] = useState(() =>
    Math.max(0, Math.min(initialChapterIndex, chapters.length - 1)),
  )

  const currentChapter = chapters[currentChapterIndex]
  const progressPercent = useMemo(
    () => computeProgress(currentChapterIndex, chapters.length),
    [currentChapterIndex, chapters.length],
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

  return (
    <section className={cn('flex min-h-screen flex-col', readerThemeStyles[readerTheme])}>
      <header className="sticky top-0 z-10 border-b border-black/10 bg-inherit/95 backdrop-blur">
        <div className="mx-auto flex w-full max-w-4xl items-center justify-between px-4 py-3">
          <Button variant="ghost" onClick={onBack}>
            <ArrowLeft className="h-4 w-4" />
            Voltar para biblioteca
          </Button>

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

      <main className="mx-auto flex w-full max-w-4xl flex-1 flex-col gap-6 px-6 py-8">
        <div className="space-y-3">
          <h1 className="font-display text-4xl leading-tight">{bookTitle}</h1>
          <div className="h-2 w-full rounded-full bg-black/10">
            <div className="h-full rounded-full bg-primary" style={{ width: `${progressPercent}%` }} />
          </div>
          <p className="text-sm opacity-80">
            Capítulo {currentChapterIndex + 1} de {chapters.length} · {progressPercent}%
          </p>
        </div>

        <article className="space-y-4">
          <h2 className="font-display text-2xl leading-tight">{currentChapter.title}</h2>
          <div className="space-y-4 text-[17px] leading-8">
            {currentChapter.content
              .split('\n\n')
              .map((paragraph) => paragraph.trim())
              .filter(Boolean)
              .map((paragraph, paragraphIndex) => (
                <p key={`${currentChapter.title}-${paragraphIndex}`}>{paragraph}</p>
              ))}
          </div>
        </article>

        <div className="mt-4 flex items-center justify-between border-t border-black/10 pt-4">
          <Button
            variant="outline"
            onClick={() => setCurrentChapterIndex((current) => Math.max(0, current - 1))}
            disabled={currentChapterIndex === 0}
          >
            <ChevronLeft className="h-4 w-4" />
            Anterior
          </Button>

          <Button
            variant="outline"
            onClick={() => setCurrentChapterIndex((current) => Math.min(chapters.length - 1, current + 1))}
            disabled={currentChapterIndex === chapters.length - 1}
          >
            Próximo
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </main>
    </section>
  )
}
