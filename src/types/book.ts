export type Book = {
  id: number
  title: string
  author: string
  format: string
  year: number
  progress: number
  description: string
  coverImageData?: string | null
  tags: string[]
  isEpubAvailable: boolean
}
