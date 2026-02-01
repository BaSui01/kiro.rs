import { useState, useEffect } from 'react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog'
import { Button } from './ui/button'
import { Input } from './ui/input'
import { Badge } from './ui/badge'
import type { PoolStatusItem, CreatePoolRequest, UpdatePoolRequest, SchedulingMode } from '../types/api'
import { Shuffle, ArrowDownToLine, Globe, Layers } from 'lucide-react'

interface PoolDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  pool?: PoolStatusItem | null // null = 创建模式, PoolStatusItem = 编辑模式
  onSubmit: (data: CreatePoolRequest | UpdatePoolRequest) => Promise<void>
}

export function PoolDialog({ open, onOpenChange, pool, onSubmit }: PoolDialogProps) {
  const isEdit = !!pool
  const [loading, setLoading] = useState(false)
  const [formData, setFormData] = useState({
    id: '',
    name: '',
    description: '',
    schedulingMode: 'round_robin' as SchedulingMode,
    proxyUrl: '',
    proxyUsername: '',
    proxyPassword: '',
    priority: 0,
  })

  useEffect(() => {
    if (pool) {
      setFormData({
        id: pool.id,
        name: pool.name,
        description: pool.description || '',
        schedulingMode: pool.schedulingMode,
        proxyUrl: '',
        proxyUsername: '',
        proxyPassword: '',
        priority: pool.priority,
      })
    } else {
      setFormData({
        id: '',
        name: '',
        description: '',
        schedulingMode: 'round_robin',
        proxyUrl: '',
        proxyUsername: '',
        proxyPassword: '',
        priority: 0,
      })
    }
  }, [pool, open])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    try {
      if (isEdit) {
        const updateData: UpdatePoolRequest = {
          name: formData.name,
          description: formData.description || undefined,
          schedulingMode: formData.schedulingMode,
          priority: formData.priority,
        }
        if (formData.proxyUrl) {
          updateData.proxyUrl = formData.proxyUrl
          updateData.proxyUsername = formData.proxyUsername || undefined
          updateData.proxyPassword = formData.proxyPassword || undefined
        }
        await onSubmit(updateData)
      } else {
        const createData: CreatePoolRequest = {
          id: formData.id,
          name: formData.name,
          description: formData.description || undefined,
          schedulingMode: formData.schedulingMode,
          priority: formData.priority,
        }
        if (formData.proxyUrl) {
          createData.proxyUrl = formData.proxyUrl
          createData.proxyUsername = formData.proxyUsername || undefined
          createData.proxyPassword = formData.proxyPassword || undefined
        }
        await onSubmit(createData)
      }
      onOpenChange(false)
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[520px]">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="p-2 bg-gradient-to-br from-cyan-500 to-blue-600 rounded-lg">
              <Layers className="h-5 w-5 text-white" />
            </div>
            <div>
              <DialogTitle>{isEdit ? '编辑池' : '创建新池'}</DialogTitle>
              <DialogDescription>
                {isEdit ? '修改池的配置信息' : '创建一个新的凭证池来管理凭据'}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="grid gap-4 py-4">
            {/* 池 ID（仅创建时可编辑） */}
            <div className="space-y-2">
              <label htmlFor="id" className="text-sm font-medium">
                池 ID
              </label>
              <Input
                id="id"
                value={formData.id}
                onChange={(e) => setFormData({ ...formData, id: e.target.value })}
                placeholder="例如: premium, vip, team-a"
                disabled={isEdit}
                required={!isEdit}
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">唯一标识符，创建后不可修改</p>
            </div>

            {/* 池名称 */}
            <div className="space-y-2">
              <label htmlFor="name" className="text-sm font-medium">
                名称
              </label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder="例如: 高级池、VIP 专用"
                required
              />
            </div>

            {/* 描述 */}
            <div className="space-y-2">
              <label htmlFor="description" className="text-sm font-medium">
                描述 <span className="text-muted-foreground font-normal">(可选)</span>
              </label>
              <Input
                id="description"
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                placeholder="池的用途说明"
              />
            </div>

            {/* 调度模式 */}
            <div className="space-y-2">
              <label className="text-sm font-medium">调度模式</label>
              <div className="grid grid-cols-2 gap-3">
                <button
                  type="button"
                  onClick={() => setFormData({ ...formData, schedulingMode: 'round_robin' })}
                  className={`p-3 rounded-lg border-2 transition-all ${
                    formData.schedulingMode === 'round_robin'
                      ? 'border-cyan-500 bg-cyan-50 dark:bg-cyan-900/20'
                      : 'border-border hover:border-muted-foreground/50'
                  }`}
                >
                  <div className="flex items-center gap-2 mb-1">
                    <Shuffle className="h-4 w-4 text-cyan-600" />
                    <span className="font-medium">轮询模式</span>
                  </div>
                  <p className="text-xs text-muted-foreground text-left">均匀分配请求到各凭据</p>
                </button>
                <button
                  type="button"
                  onClick={() => setFormData({ ...formData, schedulingMode: 'priority_fill' })}
                  className={`p-3 rounded-lg border-2 transition-all ${
                    formData.schedulingMode === 'priority_fill'
                      ? 'border-orange-500 bg-orange-50 dark:bg-orange-900/20'
                      : 'border-border hover:border-muted-foreground/50'
                  }`}
                >
                  <div className="flex items-center gap-2 mb-1">
                    <ArrowDownToLine className="h-4 w-4 text-orange-600" />
                    <span className="font-medium">优先填充</span>
                  </div>
                  <p className="text-xs text-muted-foreground text-left">优先使用高优先级凭据</p>
                </button>
              </div>
            </div>

            {/* 优先级 */}
            <div className="space-y-2">
              <label htmlFor="priority" className="text-sm font-medium">
                优先级
              </label>
              <Input
                id="priority"
                type="number"
                min="0"
                value={formData.priority}
                onChange={(e) => setFormData({ ...formData, priority: parseInt(e.target.value) || 0 })}
              />
              <p className="text-xs text-muted-foreground">数字越小优先级越高，用于池的排序</p>
            </div>

            {/* 代理配置 */}
            <div className="space-y-3 pt-2 border-t">
              <div className="flex items-center gap-2">
                <Globe className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm font-medium">代理配置</span>
                <Badge variant="secondary" className="text-xs">可选</Badge>
              </div>

              <div className="space-y-2">
                <Input
                  id="proxyUrl"
                  value={formData.proxyUrl}
                  onChange={(e) => setFormData({ ...formData, proxyUrl: e.target.value })}
                  placeholder="代理 URL: socks5://127.0.0.1:1080"
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <Input
                  id="proxyUsername"
                  value={formData.proxyUsername}
                  onChange={(e) => setFormData({ ...formData, proxyUsername: e.target.value })}
                  placeholder="用户名 (可选)"
                />
                <Input
                  id="proxyPassword"
                  type="password"
                  value={formData.proxyPassword}
                  onChange={(e) => setFormData({ ...formData, proxyPassword: e.target.value })}
                  placeholder="密码 (可选)"
                />
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              取消
            </Button>
            <Button
              type="submit"
              disabled={loading}
              className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-600 hover:to-blue-700"
            >
              {loading ? '处理中...' : isEdit ? '保存' : '创建'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
