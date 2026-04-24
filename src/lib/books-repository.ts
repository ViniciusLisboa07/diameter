import { invoke } from '@tauri-apps/api/core'

import type { Book } from '@/types/book'

type BookDto = {
  id: number
  title: string
  author: string
  description: string
  coverImageData?: string | null
  format: string
  year: number
  progress: number
  tags: string[]
  isEpubAvailable: boolean
}

type ImportRejection = {
  fileName: string
  reason: string
}

type EpubChapter = {
  title: string
  content: string
  html?: string
}

export type EpubReadResult = {
  bookId: number
  bookTitle: string
  chapters: EpubChapter[]
  lastChapterIndex: number
  progressPercent: number
}

export type ImportBooksResult = {
  importedCount: number
  rejected: ImportRejection[]
}

export type UpdateBookMetadataInput = {
  bookId: number
  title: string
  author: string
  description: string
  tags: string[]
}

export async function listBooks(): Promise<Book[]> {
  const books = await invoke<BookDto[]>('list_books')

  return books.map((book) => ({
    id: book.id,
    title: book.title,
    author: book.author,
    description: book.description,
    coverImageData: book.coverImageData,
    format: book.format,
    year: book.year,
    progress: book.progress,
    tags: book.tags,
    isEpubAvailable: book.isEpubAvailable,
  }))
}

export async function importBooks(paths: string[]): Promise<ImportBooksResult> {
  return invoke<ImportBooksResult>('import_books', { paths })
}

export async function deleteBook(bookId: number): Promise<void> {
  await invoke('delete_book', { bookId })
}

export async function updateBookMetadata(payload: UpdateBookMetadataInput): Promise<void> {
  await invoke('update_book_metadata', { payload })
}

export async function readEpub(bookId: number): Promise<EpubReadResult> {
  return invoke<EpubReadResult>('read_epub', { bookId })
}

export async function saveReadingProgress(
  bookId: number,
  lastPosition: string,
  progressPercent: number,
): Promise<void> {
  await invoke('save_reading_progress', { bookId, lastPosition, progressPercent })
}
