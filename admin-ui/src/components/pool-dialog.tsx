import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
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
  const { t } = useTranslation()
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
              <DialogTitle>{isEdit ? t('pool.editPool') : t('pool.createPool')}</DialogTitle>
              <DialogDescription>
                {isEdit ? t('pool.editPoolDescription') : t('pool.createPoolDescription')}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="grid gap-4 py-4">
            {/* 池 ID（仅创建时可编辑） */}
            <div className="space-y-2">
              <label htmlFor="id" className="text-sm font-medium">
                {t('pool.id')}
              </label>
              <Input
                id="id"
                value={formData.id}
                onChange={(e) => setFormData({ ...formData, id: e.target.value })}
                placeholder={t('pool.idPlaceholder')}
                disabled={isEdit}
                required={!isEdit}
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">{t('pool.idHelp')}</p>
            </div>

            {/* 池名称 */}
            <div className="space-y-2">
              <label htmlFor="name" className="text-sm font-medium">
                {t('pool.name')}
              </label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder={t('pool.namePlaceholder')}
                required
              />
            </div>

            {/* 描述 */}
            <div className="space-y-2">
              <label htmlFor="description" className="text-sm font-medium">
                {t('pool.description')} <span className="text-muted-foreground font-normal">({t('pool.descriptionPlaceholder')})</span>
              </label>
              <Input
                id="description"
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                placeholder={t('pool.descriptionPlaceholder')}
              />
            </div>

            {/* 调度模式 */}
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('pool.schedulingMode')}</label>
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
                    <span className="font-medium">{t('pool.schedulingModes.round_robin')}</span>
                  </div>
                  <p className="text-xs text-muted-foreground text-left">{t('pool.roundRobinDescription')}</p>
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
                    <span className="font-medium">{t('pool.schedulingModes.priority_fill')}</span>
                  </div>
                  <p className="text-xs text-muted-foreground text-left">{t('pool.priorityFillDescription')}</p>
                </button>
              </div>
            </div>

            {/* 优先级 */}
            <div className="space-y-2">
              <label htmlFor="priority" className="text-sm font-medium">
                {t('pool.priority')}
              </label>
              <Input
                id="priority"
                type="number"
                min="0"
                value={formData.priority}
                onChange={(e) => setFormData({ ...formData, priority: parseInt(e.target.value) || 0 })}
              />
              <p className="text-xs text-muted-foreground">{t('pool.priorityHelp')}</p>
            </div>

            {/* 代理配置 */}
            <div className="space-y-3 pt-2 border-t">
              <div className="flex items-center gap-2">
                <Globe className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm font-medium">{t('pool.proxySettings')}</span>
                <Badge variant="secondary" className="text-xs">{t('pool.proxyUrlPlaceholder')}</Badge>
              </div>

              <div className="space-y-2">
                <Input
                  id="proxyUrl"
                  value={formData.proxyUrl}
                  onChange={(e) => setFormData({ ...formData, proxyUrl: e.target.value })}
                  placeholder={t('pool.proxyUrlPlaceholder')}
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <Input
                  id="proxyUsername"
                  value={formData.proxyUsername}
                  onChange={(e) => setFormData({ ...formData, proxyUsername: e.target.value })}
                  placeholder={t('pool.proxyUsernamePlaceholder')}
                />
                <Input
                  id="proxyPassword"
                  type="password"
                  value={formData.proxyPassword}
                  onChange={(e) => setFormData({ ...formData, proxyPassword: e.target.value })}
                  placeholder={t('pool.proxyPasswordPlaceholder')}
                />
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              {t('common.cancel')}
            </Button>
            <Button
              type="submit"
              disabled={loading}
              className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-600 hover:to-blue-700"
            >
              {loading ? t('common.loading') : isEdit ? t('common.save') : t('common.create')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
