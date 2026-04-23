import { BookOpenText, FolderUp, Home, Tags } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'

const menuItems = [
  { icon: Home, label: 'Biblioteca', active: true },
  { icon: FolderUp, label: 'Importar', active: false },
  { icon: Tags, label: 'Coleções', active: false },
]

export function AppSidebar() {
  return (
    <aside className="flex h-full w-full flex-col rounded-xl border bg-card/80 p-4 backdrop-blur">
      <div className="flex items-center gap-3 pb-4">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <BookOpenText className="h-5 w-5" />
        </div>
        <div>
          <p className="font-display text-lg leading-none">Diameter</p>
          <p className="text-xs text-muted-foreground">MVP desktop reader</p>
        </div>
      </div>
      <Separator />
      <nav className="mt-4 flex flex-1 flex-col gap-2">
        {menuItems.map(({ icon: Icon, label, active }) => (
          <Button
            key={label}
            variant={active ? 'default' : 'ghost'}
            className="justify-start"
            aria-current={active ? 'page' : undefined}
          >
            <Icon className="h-4 w-4" />
            {label}
          </Button>
        ))}
      </nav>
      <div className="rounded-lg border border-dashed border-border bg-background/70 p-3">
        <p className="text-xs text-muted-foreground">Arraste e solte arquivos EPUB/PDF na janela para importar.</p>
      </div>
    </aside>
  )
}
