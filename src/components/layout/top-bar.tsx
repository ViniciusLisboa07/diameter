import { LayoutGrid, List, Moon, Search, Sun } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

export type LibraryViewMode = 'grid' | 'list'

type TopBarProps = {
  theme: 'light' | 'dark'
  onToggleTheme: () => void
  viewMode: LibraryViewMode
  onChangeViewMode: (mode: LibraryViewMode) => void
}

export function TopBar({ theme, onToggleTheme, viewMode, onChangeViewMode }: TopBarProps) {
  return (
    <header className="flex flex-col gap-3 rounded-xl border bg-card/80 p-4 backdrop-blur sm:flex-row sm:items-center">
      <div className="relative flex-1">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input placeholder="Buscar por título, autor ou tag" className="pl-9" />
      </div>

      <div className="inline-flex items-center rounded-md border border-input bg-background p-1">
        <Button
          size="sm"
          variant="ghost"
          className={cn('h-8 px-2', viewMode === 'grid' && 'bg-secondary')}
          aria-label="Visualização em grade"
          onClick={() => onChangeViewMode('grid')}
        >
          <LayoutGrid className="h-4 w-4" />
        </Button>
        <Button
          size="sm"
          variant="ghost"
          className={cn('h-8 px-2', viewMode === 'list' && 'bg-secondary')}
          aria-label="Visualização em lista"
          onClick={() => onChangeViewMode('list')}
        >
          <List className="h-4 w-4" />
        </Button>
      </div>

      <Button variant="outline" size="icon" onClick={onToggleTheme} aria-label="Alternar tema">
        {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
      </Button>
    </header>
  )
}
