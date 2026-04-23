import { invoke } from '@tauri-apps/api/core'

import type { Book } from '@/types/book'

type BookDto = {
  id: number
  title: string
  author: string
  description: string
  format: string
  year: number
  progress: number
  tags: string[]
}

export async function listBooks(): Promise<Book[]> {
  const books = await invoke<BookDto[]>('list_books')

  return books.map((book) => ({
    id: book.id,
    title: book.title,
    author: book.author,
    description: book.description,
    format: book.format,
    year: book.year,
    progress: book.progress,
    tags: book.tags,
  }))
}
