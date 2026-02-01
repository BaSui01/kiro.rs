import { useState } from 'react'
import {
  ArrowLeft,
  Server,
  Database,
  Globe,
  Key,
  Plus,
  Trash2,
  Copy,
  Check,
  RefreshCw,
  Loader2,
  Link,
  Unlink,
} from 'lucide-react'
import { toast } from 'sonner'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { Badge } from '@/components/ui/badge'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  useConfig,
  useUpdateConfig,
  useApiKeys,
  useCreateApiKey,
  useUpdateApiKey,
  useDeleteApiKey,
} from '@/hooks/use-settings'
import { usePools } from '@/hooks/use-pools'
import { extractErrorMessage } from '@/lib/utils'
import type { ApiKeyItem } from '@/types/api'

interface SettingsPageProps {
  onBack: () => void
}

export function SettingsPage({ onBack }: SettingsPageProps) {
  const { data: config, isLoading: configLoading, refetch: refetchConfig } = useConfig()
  const { data: apiKeys, isLoading: keysLoading, refetch: refetchKeys } = useApiKeys()
  const { pools, loading: poolsLoading, refresh: refetchPools } = usePools()
  const updateConfig = useUpdateConfig()
  const createApiKey = useCreateApiKey()
  const updateApiKey = useUpdateApiKey()
  const deleteApiKey = useDeleteApiKey()

  // 表单状态
  const [editingConfig, setEditingConfig] = useState(false)
  const [configForm, setConfigForm] = useState({
    host: '',
    port: 0,
    region: '',
    sessionCacheMaxCapacity: 0,
    sessionCacheTtlSecs: 0,
    proxyUrl: '',
    proxyUsername: '',
    proxyPassword: '',
    apiKey: '',
  })

  // API Key 对话框状态
  const [apiKeyDialogOpen, setApiKeyDialogOpen] = useState(false)
  const [newApiKeyName, setNewApiKeyName] = useState('')
  const [newApiKeyDescription, setNewApiKeyDescription] = useState('')
  const [newApiKeyPoolId, setNewApiKeyPoolId] = useState<string>('__auto__') // 默认自动路由
  const [createdKey, setCreatedKey] = useState<string | null>(null)
  const [copiedKey, setCopiedKey] = useState(false)

  // 编辑池绑定对话框
  const [editPoolDialogOpen, setEditPoolDialogOpen] = useState(false)
  const [editingApiKey, setEditingApiKey] = useState<ApiKeyItem | null>(null)
  const [editPoolId, setEditPoolId] = useState<string>('__auto__')

  // 删除确认对话框
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [keyToDelete, setKeyToDelete] = useState<ApiKeyItem | null>(null)

  // 初始化配置表单
  const initConfigForm = () => {
    if (config) {
      setConfigForm({
        host: config.host,
        port: config.port,
        region: config.region,
        sessionCacheMaxCapacity: config.sessionCacheMaxCapacity,
        sessionCacheTtlSecs: config.sessionCacheTtlSecs,
        proxyUrl: config.proxyUrl || '',
        proxyUsername: config.proxyUsername || '',
        proxyPassword: '',
        apiKey: '',
      })
    }
  }

  // 保存配置
  const handleSaveConfig = async () => {
    // 表单验证
    if (!configForm.host.trim()) {
      toast.error('主机地址不能为空')
      return
    }

    if (configForm.port < 1 || configForm.port > 65535) {
      toast.error('端口必须在 1-65535 之间')
      return
    }

    if (!configForm.region.trim()) {
      toast.error('Region 不能为空')
      return
    }

    if (configForm.sessionCacheMaxCapacity < 0) {
      toast.error('缓存容量不能为负数')
      return
    }

    if (configForm.sessionCacheTtlSecs < 0) {
      toast.error('缓存 TTL 不能为负数')
      return
    }

    // 代理 URL 格式验证（如果填写了的话）
    if (configForm.proxyUrl && !configForm.proxyUrl.match(/^(https?|socks5):\/\/.+/)) {
      toast.error('代理地址格式不正确，应为 http://、https:// 或 socks5:// 开头')
      return
    }

    try {
      await updateConfig.mutateAsync({
        host: configForm.host,
        port: configForm.port,
        region: configForm.region,
        sessionCacheMaxCapacity: configForm.sessionCacheMaxCapacity,
        sessionCacheTtlSecs: configForm.sessionCacheTtlSecs,
        proxyUrl: configForm.proxyUrl || undefined,
        proxyUsername: configForm.proxyUsername || undefined,
        proxyPassword: configForm.proxyPassword || undefined,
        apiKey: configForm.apiKey || undefined,
      })
      toast.success('配置已保存，部分配置需要重启服务后生效')
      setEditingConfig(false)
    } catch (error) {
      toast.error(extractErrorMessage(error))
    }
  }

  // 创建 API Key
  const handleCreateApiKey = async () => {
    if (!newApiKeyName.trim()) {
      toast.error('请输入 API Key 名称')
      return
    }

    try {
      const result = await createApiKey.mutateAsync({
        name: newApiKeyName,
        description: newApiKeyDescription || undefined,
        // 必须绑定池
        poolId: newApiKeyPoolId,
      })
      setCreatedKey(result.key)
      toast.success('API Key 创建成功')
    } catch (error) {
      toast.error(extractErrorMessage(error))
    }
  }

  // 复制 Key
  const handleCopyKey = async (key: string) => {
    try {
      await navigator.clipboard.writeText(key)
      setCopiedKey(true)
      toast.success('已复制到剪贴板')
      setTimeout(() => setCopiedKey(false), 2000)
    } catch {
      toast.error('复制失败')
    }
  }

  // 关闭创建对话框
  const handleCloseApiKeyDialog = () => {
    setApiKeyDialogOpen(false)
    setNewApiKeyName('')
    setNewApiKeyDescription('')
    setNewApiKeyPoolId('__auto__') // 重置为自动路由
    setCreatedKey(null)
    setCopiedKey(false)
  }

  // 打开编辑池绑定对话框
  const handleOpenEditPoolDialog = (key: ApiKeyItem) => {
    setEditingApiKey(key)
    // poolId 必须有值，如果没有则默认自动路由
    setEditPoolId(key.poolId || '__auto__')
    setEditPoolDialogOpen(true)
  }

  // 保存池绑定
  const handleSavePoolBinding = async () => {
    if (!editingApiKey) return

    try {
      await updateApiKey.mutateAsync({
        id: editingApiKey.id,
        req: {
          // 必须绑定池
          poolId: editPoolId,
        },
      })
      toast.success(
        editPoolId === '__auto__'
          ? '已设置为自动路由'
          : '已绑定到池'
      )
      setEditPoolDialogOpen(false)
      setEditingApiKey(null)
    } catch (error) {
      toast.error(extractErrorMessage(error))
    }
  }

  // 切换 API Key 启用状态
  const handleToggleApiKey = async (key: ApiKeyItem) => {
    try {
      await updateApiKey.mutateAsync({
        id: key.id,
        req: { enabled: !key.enabled },
      })
      toast.success(key.enabled ? 'API Key 已禁用' : 'API Key 已启用')
    } catch (error) {
      toast.error(extractErrorMessage(error))
    }
  }

  // 删除 API Key
  const handleDeleteApiKey = async () => {
    if (!keyToDelete) return

    try {
      await deleteApiKey.mutateAsync(keyToDelete.id)
      toast.success('API Key 已删除')
      setDeleteDialogOpen(false)
      setKeyToDelete(null)
    } catch (error) {
      toast.error(extractErrorMessage(error))
    }
  }

  if (configLoading || keysLoading || poolsLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mx-auto mb-4"></div>
          <p className="text-muted-foreground">加载中...</p>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background">
      {/* 顶部导航 */}
      <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-14 items-center justify-between px-4 md:px-8">
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="icon" onClick={onBack}>
              <ArrowLeft className="h-5 w-5" />
            </Button>
            <span className="font-semibold">设置</span>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => {
                refetchConfig()
                refetchKeys()
                refetchPools()
                toast.success('已刷新')
              }}
            >
              <RefreshCw className="h-5 w-5" />
            </Button>
          </div>
        </div>
      </header>

      {/* 主内容 */}
      <main className="container px-4 md:px-8 py-6 space-y-6">
        {/* 服务器配置 */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Server className="h-5 w-5" />
                <CardTitle>服务器配置</CardTitle>
              </div>
              {!editingConfig ? (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    initConfigForm()
                    setEditingConfig(true)
                  }}
                >
                  编辑
                </Button>
              ) : (
                <div className="flex gap-2">
                  <Button variant="outline" size="sm" onClick={() => setEditingConfig(false)}>
                    取消
                  </Button>
                  <Button size="sm" onClick={handleSaveConfig} disabled={updateConfig.isPending}>
                    {updateConfig.isPending && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                    保存
                  </Button>
                </div>
              )}
            </div>
            <CardDescription>服务器基础配置，修改后需要重启服务</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-4 md:grid-cols-3">
              <div>
                <label className="text-sm font-medium">主机地址</label>
                {editingConfig ? (
                  <Input
                    value={configForm.host}
                    onChange={(e) => setConfigForm({ ...configForm, host: e.target.value })}
                  />
                ) : (
                  <p className="text-sm text-muted-foreground mt-1">{config?.host}</p>
                )}
              </div>
              <div>
                <label className="text-sm font-medium">端口</label>
                {editingConfig ? (
                  <Input
                    type="number"
                    value={configForm.port}
                    onChange={(e) =>
                      setConfigForm({ ...configForm, port: parseInt(e.target.value) || 0 })
                    }
                  />
                ) : (
                  <p className="text-sm text-muted-foreground mt-1">{config?.port}</p>
                )}
              </div>
              <div>
                <label className="text-sm font-medium">Region</label>
                {editingConfig ? (
                  <Input
                    value={configForm.region}
                    onChange={(e) => setConfigForm({ ...configForm, region: e.target.value })}
                  />
                ) : (
                  <p className="text-sm text-muted-foreground mt-1">{config?.region}</p>
                )}
              </div>
            </div>
            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <label className="text-sm font-medium">Kiro 版本</label>
                <p className="text-sm text-muted-foreground mt-1">{config?.kiroVersion}</p>
              </div>
              <div>
                <label className="text-sm font-medium">TLS 后端</label>
                <p className="text-sm text-muted-foreground mt-1">{config?.tlsBackend}</p>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* 缓存配置 */}
        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Database className="h-5 w-5" />
              <CardTitle>缓存配置</CardTitle>
            </div>
            <CardDescription>会话缓存配置，用于粘性会话轮询</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <label className="text-sm font-medium">缓存容量</label>
                {editingConfig ? (
                  <Input
                    type="number"
                    value={configForm.sessionCacheMaxCapacity}
                    onChange={(e) =>
                      setConfigForm({
                        ...configForm,
                        sessionCacheMaxCapacity: parseInt(e.target.value) || 0,
                      })
                    }
                  />
                ) : (
                  <p className="text-sm text-muted-foreground mt-1">
                    {config?.sessionCacheMaxCapacity.toLocaleString()} 个会话
                  </p>
                )}
              </div>
              <div>
                <label className="text-sm font-medium">缓存 TTL</label>
                {editingConfig ? (
                  <Input
                    type="number"
                    value={configForm.sessionCacheTtlSecs}
                    onChange={(e) =>
                      setConfigForm({
                        ...configForm,
                        sessionCacheTtlSecs: parseInt(e.target.value) || 0,
                      })
                    }
                  />
                ) : (
                  <p className="text-sm text-muted-foreground mt-1">
                    {config?.sessionCacheTtlSecs} 秒（{Math.round((config?.sessionCacheTtlSecs || 0) / 60)} 分钟）
                  </p>
                )}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* 代理配置 */}
        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Globe className="h-5 w-5" />
              <CardTitle>代理配置</CardTitle>
            </div>
            <CardDescription>HTTP/SOCKS5 代理配置</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-4 md:grid-cols-3">
              <div>
                <label className="text-sm font-medium">代理地址</label>
                {editingConfig ? (
                  <Input
                    placeholder="http://host:port"
                    value={configForm.proxyUrl}
                    onChange={(e) => setConfigForm({ ...configForm, proxyUrl: e.target.value })}
                  />
                ) : (
                  <p className="text-sm text-muted-foreground mt-1">
                    {config?.proxyUrl || '未配置'}
                  </p>
                )}
              </div>
              <div>
                <label className="text-sm font-medium">用户名</label>
                {editingConfig ? (
                  <Input
                    value={configForm.proxyUsername}
                    onChange={(e) =>
                      setConfigForm({ ...configForm, proxyUsername: e.target.value })
                    }
                  />
                ) : (
                  <p className="text-sm text-muted-foreground mt-1">
                    {config?.proxyUsername || '未配置'}
                  </p>
                )}
              </div>
              <div>
                <label className="text-sm font-medium">密码</label>
                {editingConfig ? (
                  <Input
                    type="password"
                    placeholder="留空则不修改"
                    value={configForm.proxyPassword}
                    onChange={(e) =>
                      setConfigForm({ ...configForm, proxyPassword: e.target.value })
                    }
                  />
                ) : (
                  <p className="text-sm text-muted-foreground mt-1">
                    {config?.proxyPassword || '未配置'}
                  </p>
                )}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* API Key 配置 */}
        {editingConfig && (
          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <Key className="h-5 w-5" />
                <CardTitle>Anthropic API Key</CardTitle>
              </div>
              <CardDescription>用于下游客户端认证的 API Key</CardDescription>
            </CardHeader>
            <CardContent>
              <div>
                <label className="text-sm font-medium">API Key</label>
                <Input
                  type="password"
                  placeholder="留空则不修改"
                  value={configForm.apiKey}
                  onChange={(e) => setConfigForm({ ...configForm, apiKey: e.target.value })}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  当前状态：{config?.hasApiKey ? '已配置' : '未配置'}
                </p>
              </div>
            </CardContent>
          </Card>
        )}

        {/* API Key 管理 */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Key className="h-5 w-5" />
                <CardTitle>API Key 管理</CardTitle>
              </div>
              <Button size="sm" onClick={() => setApiKeyDialogOpen(true)}>
                <Plus className="h-4 w-4 mr-2" />
                创建 API Key
              </Button>
            </div>
            <CardDescription>管理多个 API Key，用于不同客户端或用途</CardDescription>
          </CardHeader>
          <CardContent>
            {apiKeys && apiKeys.length > 0 ? (
              <div className="space-y-3">
                {apiKeys.map((key) => (
                  <div
                    key={key.id}
                    className="flex items-center justify-between p-3 border rounded-lg"
                  >
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <span className="font-medium">{key.name}</span>
                        <Badge variant={key.enabled ? 'success' : 'secondary'}>
                          {key.enabled ? '启用' : '禁用'}
                        </Badge>
                        {key.poolId === '__auto__' ? (
                          <Badge variant="default" className="gap-1 bg-gradient-to-r from-cyan-500 to-blue-600">
                            🔄 自动路由
                          </Badge>
                        ) : key.poolId ? (
                          <Badge variant="outline" className="gap-1">
                            <Link className="h-3 w-3" />
                            {pools.find((p) => p.id === key.poolId)?.name || key.poolId}
                          </Badge>
                        ) : (
                          <Badge variant="warning" className="gap-1">
                            ⚠️ 未绑定池
                          </Badge>
                        )}
                      </div>
                      <p className="text-sm text-muted-foreground font-mono">{key.key}</p>
                      <p className="text-xs text-muted-foreground mt-1">
                        创建于 {new Date(key.createdAt).toLocaleDateString('zh-CN')}
                        {key.description && ` · ${key.description}`}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleOpenEditPoolDialog(key)}
                        title="编辑池绑定"
                      >
                        {key.poolId ? <Link className="h-4 w-4" /> : <Unlink className="h-4 w-4" />}
                      </Button>
                      <Switch checked={key.enabled} onCheckedChange={() => handleToggleApiKey(key)} />
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => {
                          setKeyToDelete(key)
                          setDeleteDialogOpen(true)
                        }}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-center text-muted-foreground py-8">暂无 API Key</p>
            )}
          </CardContent>
        </Card>
      </main>

      {/* 创建 API Key 对话框 */}
      <Dialog open={apiKeyDialogOpen} onOpenChange={handleCloseApiKeyDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{createdKey ? 'API Key 已创建' : '创建 API Key'}</DialogTitle>
            <DialogDescription>
              {createdKey
                ? '请立即复制保存，关闭后将无法再次查看完整 Key'
                : '创建一个新的 API Key'}
            </DialogDescription>
          </DialogHeader>

          {createdKey ? (
            <div className="space-y-4">
              <div className="p-3 bg-muted rounded-lg">
                <p className="text-sm font-mono break-all">{createdKey}</p>
              </div>
              <Button className="w-full" onClick={() => handleCopyKey(createdKey)}>
                {copiedKey ? (
                  <>
                    <Check className="h-4 w-4 mr-2" />
                    已复制
                  </>
                ) : (
                  <>
                    <Copy className="h-4 w-4 mr-2" />
                    复制 Key
                  </>
                )}
              </Button>
            </div>
          ) : (
            <>
              <div className="space-y-4">
                <div>
                  <label className="text-sm font-medium">名称 *</label>
                  <Input
                    placeholder="例如：Production、Development"
                    value={newApiKeyName}
                    onChange={(e) => setNewApiKeyName(e.target.value)}
                  />
                </div>
                <div>
                  <label className="text-sm font-medium">描述</label>
                  <Input
                    placeholder="可选的描述信息"
                    value={newApiKeyDescription}
                    onChange={(e) => setNewApiKeyDescription(e.target.value)}
                  />
                </div>
                <div>
                  <label className="text-sm font-medium">绑定池</label>
                  <Select value={newApiKeyPoolId} onValueChange={setNewApiKeyPoolId}>
                    <SelectTrigger>
                      <SelectValue placeholder="选择池（可选）" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="__auto__">🔄 自动路由（按优先级遍历所有池）</SelectItem>
                      {pools.map((pool) => (
                        <SelectItem key={pool.id} value={pool.id}>
                          {pool.name} ({pool.id})
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground mt-1">
                    不选择则使用默认池
                  </p>
                </div>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={handleCloseApiKeyDialog}>
                  取消
                </Button>
                <Button onClick={handleCreateApiKey} disabled={createApiKey.isPending}>
                  {createApiKey.isPending && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                  创建
                </Button>
              </DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>

      {/* 删除确认对话框 */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>确认删除</DialogTitle>
            <DialogDescription>
              确定要删除 API Key "{keyToDelete?.name}" 吗？此操作不可撤销。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialogOpen(false)}>
              取消
            </Button>
            <Button variant="destructive" onClick={handleDeleteApiKey} disabled={deleteApiKey.isPending}>
              {deleteApiKey.isPending && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              删除
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 编辑池绑定对话框 */}
      <Dialog open={editPoolDialogOpen} onOpenChange={setEditPoolDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>编辑池绑定</DialogTitle>
            <DialogDescription>
              为 API Key "{editingApiKey?.name}" 选择要绑定的池
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <label className="text-sm font-medium">绑定池</label>
              <Select value={editPoolId} onValueChange={setEditPoolId}>
                <SelectTrigger>
                  <SelectValue placeholder="选择池" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__auto__">🔄 自动路由（按优先级遍历所有池）</SelectItem>
                  {pools.map((pool) => (
                    <SelectItem key={pool.id} value={pool.id}>
                      {pool.name} ({pool.id})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground mt-1">
                选择自动路由或绑定到特定池
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditPoolDialogOpen(false)}>
              取消
            </Button>
            <Button onClick={handleSavePoolBinding} disabled={updateApiKey.isPending}>
              {updateApiKey.isPending && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              保存
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
