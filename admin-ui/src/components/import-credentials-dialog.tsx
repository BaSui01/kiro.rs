import { useState, useRef, useEffect } from 'react'
import { Upload, FileJson, AlertCircle, CheckCircle2 } from 'lucide-react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { importCredentials } from '@/api/credentials'
import { usePools } from '@/hooks/use-pools'
import type { IdcCredentialItem, ImportCredentialsResponse } from '@/types/api'

interface ImportCredentialsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** 默认选中的池ID，用于从特定池触发导入时预选目标池 */
  defaultPoolId?: string
}

export function ImportCredentialsDialog({ open, onOpenChange, defaultPoolId = 'default' }: ImportCredentialsDialogProps) {
  const [selectedFiles, setSelectedFiles] = useState<File[]>([])
  const [parsedCredentials, setParsedCredentials] = useState<IdcCredentialItem[]>([])
  const [parseError, setParseError] = useState<string | null>(null)
  const [importResult, setImportResult] = useState<ImportCredentialsResponse | null>(null)
  const [selectedPoolId, setSelectedPoolId] = useState<string>(defaultPoolId) // 使用传入的默认池ID
  const fileInputRef = useRef<HTMLInputElement>(null)
  const queryClient = useQueryClient()
  const { pools } = usePools()

  // 当对话框打开时，同步 defaultPoolId 到 selectedPoolId
  // 这样从不同池触发导入时，会自动选中对应的池 🎯
  useEffect(() => {
    if (open) {
      setSelectedPoolId(defaultPoolId)
    }
  }, [open, defaultPoolId])

  const importMutation = useMutation({
    mutationFn: importCredentials,
    onSuccess: (data) => {
      setImportResult(data)
      if (data.importedCount > 0) {
        toast.success(`成功导入 ${data.importedCount} 个凭据`)
        queryClient.invalidateQueries({ queryKey: ['credentials'] })
        queryClient.invalidateQueries({ queryKey: ['pools'] })
      }
      if (data.skippedCount > 0) {
        toast.warning(`跳过 ${data.skippedCount} 个无效凭据`)
      }
    },
    onError: (error: Error) => {
      toast.error(`导入失败: ${error.message}`)
    },
  })

  /**
   * 从 Kiro Account Manager 导出格式中提取凭证
   * 支持格式：{ version, account: { credentials: { refreshToken, ... } } }
   */
  const extractFromKiroAccountManager = (parsed: Record<string, unknown>): IdcCredentialItem | null => {
    // 检查是否是 Kiro Account Manager 导出格式
    if (parsed.version && parsed.account && typeof parsed.account === 'object') {
      const account = parsed.account as Record<string, unknown>
      if (account.credentials && typeof account.credentials === 'object') {
        const creds = account.credentials as Record<string, unknown>
        // 提取必要字段
        if (creds.refreshToken) {
          return {
            email: account.email as string | undefined,
            label: (account.nickname as string) || (account.email as string) || undefined,
            refreshToken: creds.refreshToken as string,
            accessToken: creds.accessToken as string | undefined,
            expiresAt: creds.expiresAt ? new Date(creds.expiresAt as number).toISOString() : undefined,
            clientId: creds.clientId as string | undefined,
            clientSecret: creds.clientSecret as string | undefined,
            region: creds.region as string | undefined,
            provider: creds.provider as string | undefined,
          }
        }
      }
    }
    return null
  }

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || [])
    if (files.length === 0) return

    setSelectedFiles(files)
    setParseError(null)
    setImportResult(null)

    const allCredentials: IdcCredentialItem[] = []

    for (const file of files) {
      try {
        const content = await file.text()
        const parsed = JSON.parse(content)

        // 支持数组格式
        if (Array.isArray(parsed)) {
          for (const item of parsed) {
            // 尝试从 Kiro Account Manager 格式提取
            const extracted = extractFromKiroAccountManager(item)
            if (extracted) {
              allCredentials.push(extracted)
            } else {
              allCredentials.push(item)
            }
          }
        } else if (typeof parsed === 'object' && parsed !== null) {
          // 尝试从 Kiro Account Manager 格式提取
          const extracted = extractFromKiroAccountManager(parsed)
          if (extracted) {
            allCredentials.push(extracted)
          } else {
            // 直接作为凭证格式
            allCredentials.push(parsed)
          }
        }
      } catch (err) {
        setParseError(`解析文件 ${file.name} 失败: ${(err as Error).message}`)
        return
      }
    }

    setParsedCredentials(allCredentials)
  }

  const handleImport = () => {
    if (parsedCredentials.length === 0) {
      toast.error('没有可导入的凭据')
      return
    }

    importMutation.mutate({
      credentials: parsedCredentials,
      poolId: selectedPoolId, // 始终传递 poolId，包括 default
    })
  }

  const handleClose = () => {
    setSelectedFiles([])
    setParsedCredentials([])
    setParseError(null)
    setImportResult(null)
    setSelectedPoolId(defaultPoolId) // 重置为传入的默认池ID，而不是硬编码 'default'
    onOpenChange(false)
  }

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
  }

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()

    const files = Array.from(e.dataTransfer.files).filter(
      (f) => f.name.endsWith('.json')
    )

    if (files.length === 0) {
      toast.error('请拖放 JSON 文件')
      return
    }

    // 模拟 file input 的行为
    const dataTransfer = new DataTransfer()
    files.forEach((f) => dataTransfer.items.add(f))
    if (fileInputRef.current) {
      fileInputRef.current.files = dataTransfer.files
      handleFileSelect({ target: fileInputRef.current } as React.ChangeEvent<HTMLInputElement>)
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Upload className="h-5 w-5" />
            导入凭据
          </DialogTitle>
          <DialogDescription>
            支持从 Kiro Account Manager 导出的 JSON 文件导入凭据（IdC/Builder-ID/IAM 格式）
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* 目标池选择 */}
          <div className="space-y-2">
            <label className="text-sm font-medium">导入到池</label>
            <Select value={selectedPoolId} onValueChange={setSelectedPoolId}>
              <SelectTrigger>
                <SelectValue placeholder="选择目标池" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="default">默认池 (default)</SelectItem>
                {pools
                  .filter((p) => p.id !== 'default')
                  .map((pool) => (
                    <SelectItem key={pool.id} value={pool.id}>
                      {pool.name} ({pool.id})
                    </SelectItem>
                  ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              选择要将凭据导入到哪个池
            </p>
          </div>

          {/* 文件上传区域 */}
          <div
            className="border-2 border-dashed rounded-lg p-6 text-center cursor-pointer hover:border-primary transition-colors"
            onClick={() => fileInputRef.current?.click()}
            onDragOver={handleDragOver}
            onDrop={handleDrop}
          >
            <input
              ref={fileInputRef}
              type="file"
              accept=".json"
              multiple
              className="hidden"
              onChange={handleFileSelect}
            />
            <FileJson className="h-10 w-10 mx-auto mb-2 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">
              点击选择或拖放 JSON 文件
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              支持多文件选择
            </p>
          </div>

          {/* 已选择的文件 */}
          {selectedFiles.length > 0 && (
            <div className="text-sm">
              <p className="font-medium mb-1">已选择 {selectedFiles.length} 个文件:</p>
              <ul className="text-muted-foreground space-y-1">
                {selectedFiles.map((f, i) => (
                  <li key={i} className="truncate">• {f.name}</li>
                ))}
              </ul>
            </div>
          )}

          {/* 解析错误 */}
          {parseError && (
            <div className="flex items-start gap-2 p-3 bg-red-50 dark:bg-red-950 rounded-lg text-red-600 dark:text-red-400 text-sm">
              <AlertCircle className="h-4 w-4 mt-0.5 flex-shrink-0" />
              <span>{parseError}</span>
            </div>
          )}

          {/* 解析结果预览 */}
          {parsedCredentials.length > 0 && !parseError && (
            <div className="p-3 bg-muted rounded-lg">
              <p className="text-sm font-medium mb-2">
                解析到 {parsedCredentials.length} 个凭据:
              </p>
              <ul className="text-xs text-muted-foreground space-y-1 max-h-32 overflow-y-auto">
                {parsedCredentials.slice(0, 10).map((cred, i) => (
                  <li key={i} className="truncate">
                    • {cred.label || cred.email || `凭据 ${i + 1}`}
                    {cred.clientId ? ' (IdC)' : ' (Social)'}
                  </li>
                ))}
                {parsedCredentials.length > 10 && (
                  <li className="text-muted-foreground">
                    ... 还有 {parsedCredentials.length - 10} 个
                  </li>
                )}
              </ul>
            </div>
          )}

          {/* 导入结果 */}
          {importResult && (
            <div className={`p-3 rounded-lg text-sm ${
              importResult.importedCount > 0
                ? 'bg-green-50 dark:bg-green-950 text-green-600 dark:text-green-400'
                : 'bg-yellow-50 dark:bg-yellow-950 text-yellow-600 dark:text-yellow-400'
            }`}>
              <div className="flex items-center gap-2 mb-2">
                <CheckCircle2 className="h-4 w-4" />
                <span className="font-medium">{importResult.message}</span>
              </div>
              {importResult.skippedItems.length > 0 && (
                <div className="mt-2">
                  <p className="text-xs font-medium mb-1">跳过的凭据:</p>
                  <ul className="text-xs space-y-0.5 max-h-24 overflow-y-auto">
                    {importResult.skippedItems.map((item, i) => (
                      <li key={i} className="truncate">• {item}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}

          {/* 操作按钮 */}
          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={handleClose}>
              {importResult ? '关闭' : '取消'}
            </Button>
            {!importResult && (
              <Button
                onClick={handleImport}
                disabled={parsedCredentials.length === 0 || importMutation.isPending}
              >
                {importMutation.isPending ? '导入中...' : `导入 ${parsedCredentials.length} 个凭据`}
              </Button>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
