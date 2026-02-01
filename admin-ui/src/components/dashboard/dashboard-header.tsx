import { RefreshCw, LogOut, Moon, Sun, Server, Settings } from 'lucide-react'
import { Button } from '@/components/ui/button'

export interface DashboardHeaderProps {
  darkMode: boolean
  onToggleDarkMode: () => void
  onRefresh: () => void
  onSettings: () => void
  onLogout: () => void
}

export function DashboardHeader({
  darkMode,
  onToggleDarkMode,
  onRefresh,
  onSettings,
  onLogout,
}: DashboardHeaderProps) {
  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-14 items-center justify-between px-4 md:px-8">
        <div className="flex items-center gap-2">
          <Server className="h-5 w-5" />
          <span className="font-semibold">Kiro Admin</span>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="icon" onClick={onToggleDarkMode}>
            {darkMode ? <Sun className="h-5 w-5" /> : <Moon className="h-5 w-5" />}
          </Button>
          <Button variant="ghost" size="icon" onClick={onRefresh}>
            <RefreshCw className="h-5 w-5" />
          </Button>
          <Button variant="ghost" size="icon" onClick={onSettings}>
            <Settings className="h-5 w-5" />
          </Button>
          <Button variant="ghost" size="icon" onClick={onLogout}>
            <LogOut className="h-5 w-5" />
          </Button>
        </div>
      </div>
    </header>
  )
}
