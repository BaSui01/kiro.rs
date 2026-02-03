import { RefreshCw, LogOut, Moon, Sun, Zap, Settings } from "lucide-react";
import { Button } from "@/components/ui/button";
import { LanguageSwitcher } from "@/components/language-switcher";

export interface DashboardHeaderProps {
  darkMode: boolean;
  onToggleDarkMode: () => void;
  onRefresh: () => void;
  onSettings: () => void;
  onLogout: () => void;
}

export function DashboardHeader({
  darkMode,
  onToggleDarkMode,
  onRefresh,
  onSettings,
  onLogout,
}: DashboardHeaderProps) {
  return (
    <header className="sticky top-0 z-50 w-full border-b bg-gradient-to-r from-background via-background to-background/95 backdrop-blur-lg supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-16 items-center justify-between px-4 md:px-8">
        <div className="flex items-center gap-3">
          <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-primary shadow-lg shadow-primary/25">
            <Zap className="h-5 w-5 text-primary-foreground" />
          </div>
          <div className="flex flex-col">
            <span className="font-bold text-lg text-primary">Kiro Admin</span>
            <span className="text-xs text-muted-foreground">
              凭证池管理系统
            </span>
          </div>
        </div>
        <div className="flex items-center gap-1">
          <LanguageSwitcher />
          <Button
            variant="ghost"
            size="icon"
            onClick={onToggleDarkMode}
            className="rounded-xl hover:bg-muted/80 transition-all"
          >
            {darkMode ? (
              <Sun className="h-5 w-5 text-amber-500" />
            ) : (
              <Moon className="h-5 w-5 text-slate-600" />
            )}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={onRefresh}
            className="rounded-xl hover:bg-muted/80 transition-all"
          >
            <RefreshCw className="h-5 w-5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={onSettings}
            className="rounded-xl hover:bg-muted/80 transition-all"
          >
            <Settings className="h-5 w-5" />
          </Button>
          <div className="w-px h-6 bg-border mx-2" />
          <Button
            variant="ghost"
            size="icon"
            onClick={onLogout}
            className="rounded-xl hover:bg-red-500/10 hover:text-red-500 transition-all"
          >
            <LogOut className="h-5 w-5" />
          </Button>
        </div>
      </div>
    </header>
  );
}
