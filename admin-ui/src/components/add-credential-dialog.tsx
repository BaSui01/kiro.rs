import { useState } from 'react'
import { toast } from 'sonner'
import { ChevronDown, ChevronRight } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useAddCredential } from '@/hooks/use-credentials'
import { extractErrorMessage } from '@/lib/utils'

interface AddCredentialDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

type AuthMethod = 'social' | 'idc'

export function AddCredentialDialog({ open, onOpenChange }: AddCredentialDialogProps) {
  const [refreshToken, setRefreshToken] = useState('')
  const [authMethod, setAuthMethod] = useState<AuthMethod>('social')
  const [region, setRegion] = useState('')
  const [clientId, setClientId] = useState('')
  const [clientSecret, setClientSecret] = useState('')
  const [priority, setPriority] = useState('0')
  // 代理配置
  const [showProxyConfig, setShowProxyConfig] = useState(false)
  const [proxyUrl, setProxyUrl] = useState('')
  const [proxyUsername, setProxyUsername] = useState('')
  const [proxyPassword, setProxyPassword] = useState('')

  const { mutate, isPending } = useAddCredential()

  const resetForm = () => {
    setRefreshToken('')
    setAuthMethod('social')
    setRegion('')
    setClientId('')
    setClientSecret('')
    setPriority('0')
    setShowProxyConfig(false)
    setProxyUrl('')
    setProxyUsername('')
    setProxyPassword('')
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    // 验证必填字段
    if (!refreshToken.trim()) {
      toast.error('请输入 Refresh Token')
      return
    }

    // IdC/Builder-ID/IAM 需要额外字段
    if (authMethod === 'idc' && (!clientId.trim() || !clientSecret.trim())) {
      toast.error('IdC/Builder-ID/IAM 认证需要填写 Client ID 和 Client Secret')
      return
    }

    // 代理 URL 格式验证
    if (proxyUrl.trim() && !proxyUrl.match(/^(https?|socks5):\/\/.+/)) {
      toast.error('代理地址格式不正确，应为 http://、https:// 或 socks5:// 开头')
      return
    }

    mutate(
      {
        refreshToken: refreshToken.trim(),
        authMethod,
        region: region.trim() || undefined,
        clientId: clientId.trim() || undefined,
        clientSecret: clientSecret.trim() || undefined,
        priority: parseInt(priority) || 0,
        proxyUrl: proxyUrl.trim() || undefined,
        proxyUsername: proxyUsername.trim() || undefined,
        proxyPassword: proxyPassword.trim() || undefined,
      },
      {
        onSuccess: (data) => {
          toast.success(data.message)
          onOpenChange(false)
          resetForm()
        },
        onError: (error: unknown) => {
          toast.error(`添加失败: ${extractErrorMessage(error)}`)
        },
      }
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>添加凭据</DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit}>
          <div className="space-y-4 py-4">
            {/* Refresh Token */}
            <div className="space-y-2">
              <label htmlFor="refreshToken" className="text-sm font-medium">
                Refresh Token <span className="text-red-500">*</span>
              </label>
              <Input
                id="refreshToken"
                type="password"
                placeholder="请输入 Refresh Token"
                value={refreshToken}
                onChange={(e) => setRefreshToken(e.target.value)}
                disabled={isPending}
              />
            </div>

            {/* 认证方式 */}
            <div className="space-y-2">
              <label htmlFor="authMethod" className="text-sm font-medium">
                认证方式
              </label>
              <select
                id="authMethod"
                value={authMethod}
                onChange={(e) => setAuthMethod(e.target.value as AuthMethod)}
                disabled={isPending}
                className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
              >
                <option value="social">Social</option>
                <option value="idc">IdC/Builder-ID/IAM</option>
              </select>
            </div>

            <div className="space-y-2">
              <label htmlFor="region" className="text-sm font-medium">
                刷新 Token 地域
              </label>
              <Input
                id="region"
                placeholder="例如 us-east-1（留空则使用全局 region）"
                value={region}
                onChange={(e) => setRegion(e.target.value)}
                disabled={isPending}
              />
            </div>

            {/* IdC/Builder-ID/IAM 额外字段 */}
            {authMethod === 'idc' && (
              <>
                <div className="space-y-2">
                  <label htmlFor="clientId" className="text-sm font-medium">
                    Client ID <span className="text-red-500">*</span>
                  </label>
                  <Input
                    id="clientId"
                    placeholder="请输入 Client ID"
                    value={clientId}
                    onChange={(e) => setClientId(e.target.value)}
                    disabled={isPending}
                  />
                </div>
                <div className="space-y-2">
                  <label htmlFor="clientSecret" className="text-sm font-medium">
                    Client Secret <span className="text-red-500">*</span>
                  </label>
                  <Input
                    id="clientSecret"
                    type="password"
                    placeholder="请输入 Client Secret"
                    value={clientSecret}
                    onChange={(e) => setClientSecret(e.target.value)}
                    disabled={isPending}
                  />
                </div>
              </>
            )}

            {/* 优先级 */}
            <div className="space-y-2">
              <label htmlFor="priority" className="text-sm font-medium">
                优先级
              </label>
              <Input
                id="priority"
                type="number"
                min="0"
                placeholder="数字越小优先级越高"
                value={priority}
                onChange={(e) => setPriority(e.target.value)}
                disabled={isPending}
              />
              <p className="text-xs text-muted-foreground">
                数字越小优先级越高，默认为 0
              </p>
            </div>

            {/* 代理配置（可折叠） */}
            <div className="border rounded-lg">
              <button
                type="button"
                className="flex items-center justify-between w-full p-3 text-sm font-medium hover:bg-muted/50 transition-colors"
                onClick={() => setShowProxyConfig(!showProxyConfig)}
              >
                <span>代理配置（可选）</span>
                {showProxyConfig ? (
                  <ChevronDown className="h-4 w-4" />
                ) : (
                  <ChevronRight className="h-4 w-4" />
                )}
              </button>
              {showProxyConfig && (
                <div className="p-3 pt-0 space-y-3 border-t">
                  <p className="text-xs text-muted-foreground">
                    凭据级代理优先级高于池级和全局代理
                  </p>
                  <div className="space-y-2">
                    <label htmlFor="proxyUrl" className="text-sm font-medium">
                      代理地址
                    </label>
                    <Input
                      id="proxyUrl"
                      placeholder="例如 socks5://127.0.0.1:1080"
                      value={proxyUrl}
                      onChange={(e) => setProxyUrl(e.target.value)}
                      disabled={isPending}
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="space-y-2">
                      <label htmlFor="proxyUsername" className="text-sm font-medium">
                        用户名
                      </label>
                      <Input
                        id="proxyUsername"
                        placeholder="可选"
                        value={proxyUsername}
                        onChange={(e) => setProxyUsername(e.target.value)}
                        disabled={isPending}
                      />
                    </div>
                    <div className="space-y-2">
                      <label htmlFor="proxyPassword" className="text-sm font-medium">
                        密码
                      </label>
                      <Input
                        id="proxyPassword"
                        type="password"
                        placeholder="可选"
                        value={proxyPassword}
                        onChange={(e) => setProxyPassword(e.target.value)}
                        disabled={isPending}
                      />
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={isPending}
            >
              取消
            </Button>
            <Button type="submit" disabled={isPending}>
              {isPending ? '添加中...' : '添加'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
