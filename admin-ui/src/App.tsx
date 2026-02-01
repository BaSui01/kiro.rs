import { useState, useEffect } from 'react'
import { storage } from '@/lib/storage'
import { initCsrfToken } from '@/api/credentials'
import { LoginPage } from '@/components/login-page'
import { UnifiedDashboard } from '@/components/unified-dashboard'
import { SettingsPage } from '@/components/settings-page'
import { Toaster } from '@/components/ui/sonner'

type Page = 'dashboard' | 'settings'

function App() {
  const [isLoggedIn, setIsLoggedIn] = useState(false)
  const [currentPage, setCurrentPage] = useState<Page>('dashboard')

  useEffect(() => {
    // 检查是否已经有保存的 API Key
    if (storage.getApiKey()) {
      setIsLoggedIn(true)
      // 初始化 CSRF Token
      initCsrfToken()
    }
  }, [])

  const handleLogin = async () => {
    setIsLoggedIn(true)
    // 登录成功后初始化 CSRF Token
    await initCsrfToken()
  }

  const handleLogout = () => {
    setIsLoggedIn(false)
    setCurrentPage('dashboard')
  }

  const renderPage = () => {
    if (!isLoggedIn) {
      return <LoginPage onLogin={handleLogin} />
    }

    switch (currentPage) {
      case 'settings':
        return <SettingsPage onBack={() => setCurrentPage('dashboard')} />
      case 'dashboard':
      default:
        return (
          <UnifiedDashboard
            onLogout={handleLogout}
            onSettings={() => setCurrentPage('settings')}
          />
        )
    }
  }

  return (
    <>
      {renderPage()}
      <Toaster position="top-right" />
    </>
  )
}

export default App
