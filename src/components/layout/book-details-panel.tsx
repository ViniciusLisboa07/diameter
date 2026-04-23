import { useEffect, useMemo, useState } from 'react'

import type { Book } from '@/types/book'

import { BookCover } from '@/components/layout/book-cover'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Separator } from '@/components/ui/separator'
import { Textarea } from '@/components/ui/textarea'

type MetadataDraft = {
  title: string
  author: string
  description: string
  tags: string[]
}

type BookDetailsPanelProps = {
  book: Book
  onSaveMetadata: (bookId: number, draft: MetadataDraft) => Promise<void>
}

function parseTags(tagsInput: string): string[] {
  const normalized = tagsInput
    .split(',')
    .map((tag) => tag.trim())
    .filter(Boolean)

  return [...new Set(normalized.map((tag) => tag.toLowerCase()))]
}

function areTagArraysEqual(left: string[], right: string[]): boolean {
  if (left.length !== right.length) {
    return false
  }

  return left.every((tag, index) => tag === right[index])
}

export function BookDetailsPanel({ book, onSaveMetadata }: BookDetailsPanelProps) {
  const [title, setTitle] = useState(book.title)
  const [author, setAuthor] = useState(book.author)
  const [description, setDescription] = useState(book.description)
  const [tagsInput, setTagsInput] = useState(book.tags.join(', '))
  const [isSaving, setIsSaving] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)
  const [saveSuccess, setSaveSuccess] = useState<string | null>(null)

  useEffect(() => {
    setTitle(book.title)
    setAuthor(book.author)
    setDescription(book.description)
    setTagsInput(book.tags.join(', '))
    setSaveError(null)
    setSaveSuccess(null)
  }, [book])

  const parsedTags = useMemo(() => parseTags(tagsInput), [tagsInput])

  const isDirty = useMemo(() => {
    const normalizedTitle = title.trim() || 'Livro sem título'
    const normalizedAuthor = author.trim() || 'Autor desconhecido'
    const normalizedDescription = description.trim()

    return (
      normalizedTitle !== book.title ||
      normalizedAuthor !== book.author ||
      normalizedDescription !== book.description ||
      !areTagArraysEqual(parsedTags, book.tags.map((tag) => tag.toLowerCase()))
    )
  }, [title, author, description, parsedTags, book])

  const handleSave = async () => {
    if (!isDirty || isSaving) {
      return
    }

    setIsSaving(true)
    setSaveError(null)
    setSaveSuccess(null)

    try {
      const draft: MetadataDraft = {
        title: title.trim() || 'Livro sem título',
        author: author.trim() || 'Autor desconhecido',
        description: description.trim(),
        tags: parsedTags,
      }

      await onSaveMetadata(book.id, draft)
      setSaveSuccess('Metadados salvos.')
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Falha ao salvar metadados.'
      setSaveError(message)
    } finally {
      setIsSaving(false)
    }
  }

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

          <div className="space-y-3">
            <p className="text-sm font-medium text-foreground">Metadados</p>

            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">Título</p>
              <Input value={title} onChange={(event) => setTitle(event.target.value)} />
            </div>

            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">Autor</p>
              <Input value={author} onChange={(event) => setAuthor(event.target.value)} />
            </div>

            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">Descrição</p>
              <Textarea value={description} onChange={(event) => setDescription(event.target.value)} />
            </div>

            <div className="space-y-1">
              <p className="text-xs text-muted-foreground">Tags (separadas por vírgula)</p>
              <Input value={tagsInput} onChange={(event) => setTagsInput(event.target.value)} />
            </div>

            <Button onClick={handleSave} disabled={!isDirty || isSaving} className="w-full">
              {isSaving ? 'Salvando...' : 'Salvar metadados'}
            </Button>

            {saveError && <p className="text-xs text-red-500">{saveError}</p>}
            {saveSuccess && <p className="text-xs text-emerald-600 dark:text-emerald-400">{saveSuccess}</p>}
          </div>
        </CardContent>
      </Card>
    </aside>
  )
}
