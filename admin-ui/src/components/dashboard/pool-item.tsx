import { useState, useEffect } from "react";
import {
  Plus,
  Upload,
  Shuffle,
  ArrowDownToLine,
  ChevronDown,
  ChevronRight,
  Shield,
  Wifi,
  Users,
  Key,
  Sparkles,
  ArrowRightLeft,
  Loader2,
} from "lucide-react";
import { toast } from "sonner";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { CredentialCard } from "@/components/credential-card";
import type {
  PoolStatusItem,
  SchedulingMode,
  CredentialStatusItem,
  PoolCredentialsResponse,
} from "@/types/api";

export interface PoolItemProps {
  pool: PoolStatusItem;
  expanded: boolean;
  onToggleExpand: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onToggleEnabled: (enabled: boolean) => void;
  credentials: CredentialStatusItem[];
  onViewBalance: (id: number) => void;
  onAddCredential: () => void;
  onImportCredentials: () => void;
  // 新增：获取池凭证列表的方法
  fetchPoolCredentials?: (poolId: string) => Promise<PoolCredentialsResponse>;
  // 新增：转移凭证的方法
  onTransferCredential?: (
    credentialId: number,
    targetPoolId: string
  ) => Promise<void>;
  // 新增：所有池列表（用于转移目标选择）
  allPools?: PoolStatusItem[];
}

export function PoolItem({
  pool,
  expanded,
  onToggleExpand,
  onEdit,
  onDelete,
  onToggleEnabled,
  credentials,
  onViewBalance,
  onAddCredential,
  onImportCredentials,
  fetchPoolCredentials,
  onTransferCredential,
  allPools = [],
}: PoolItemProps) {
  const isDefault = pool.id === "default";
  const schedulingModeLabel =
    pool.schedulingMode === "round_robin" ? "轮询" : "优先填充";
  const SchedulingModeIcon =
    pool.schedulingMode === "round_robin" ? Shuffle : ArrowDownToLine;

  // 非默认池的凭证列表状态
  const [poolCredentials, setPoolCredentials] = useState<
    CredentialStatusItem[]
  >([]);
  const [loadingCredentials, setLoadingCredentials] = useState(false);
  const [credentialsLoaded, setCredentialsLoaded] = useState(false);

  // 转移凭证状态
  const [transferringId, setTransferringId] = useState<number | null>(null);
  const [selectedTargetPool, setSelectedTargetPool] = useState<string>("");

  // 当展开非默认池时，加载凭证列表
  useEffect(() => {
    if (expanded && !isDefault && !credentialsLoaded && fetchPoolCredentials) {
      setLoadingCredentials(true);
      fetchPoolCredentials(pool.id)
        .then((response) => {
          setPoolCredentials(response.credentials);
          setCredentialsLoaded(true);
        })
        .catch((err) => {
          toast.error(`加载凭证列表失败: ${err.message}`);
        })
        .finally(() => {
          setLoadingCredentials(false);
        });
    }
  }, [expanded, isDefault, credentialsLoaded, fetchPoolCredentials, pool.id]);

  // 当池折叠时，重置加载状态（下次展开时重新加载）
  useEffect(() => {
    if (!expanded) {
      setCredentialsLoaded(false);
    }
  }, [expanded]);

  // 处理凭证转移
  const handleTransfer = async (credentialId: number) => {
    if (!selectedTargetPool || !onTransferCredential) return;

    setTransferringId(credentialId);
    try {
      await onTransferCredential(credentialId, selectedTargetPool);
      toast.success(`凭证 #${credentialId} 已转移到池 ${selectedTargetPool}`);
      // 重新加载凭证列表
      setCredentialsLoaded(false);
      setSelectedTargetPool("");
    } catch (err) {
      toast.error(`转移失败: ${(err as Error).message}`);
    } finally {
      setTransferringId(null);
    }
  };

  // 获取可转移的目标池列表（排除当前池）
  const targetPools = allPools.filter((p) => p.id !== pool.id);

  // 实际显示的凭证列表
  const displayCredentials = isDefault ? credentials : poolCredentials;

  return (
    <Card
      className={`overflow-hidden border-0 shadow-sm hover:shadow-md transition-all duration-300 ${
        !pool.enabled ? "opacity-60" : ""
      }`}
    >
      <div
        className="flex items-center justify-between p-5 cursor-pointer hover:bg-muted/30 transition-colors"
        onClick={onToggleExpand}
      >
        <div className="flex items-center gap-4">
          <div
            className={`flex items-center justify-center w-12 h-12 rounded-xl transition-all ${
              expanded ? "bg-primary shadow-lg shadow-primary/25" : "bg-muted"
            }`}
          >
            {expanded ? (
              <ChevronDown className="h-5 w-5 text-white" />
            ) : (
              <ChevronRight className="h-5 w-5 text-muted-foreground" />
            )}
          </div>

          <div>
            <div className="flex items-center gap-2 mb-1">
              <span className="font-semibold text-lg">{pool.name}</span>
              <Badge variant="outline" className="text-xs font-mono">
                {pool.id}
              </Badge>
              {isDefault && (
                <Badge className="text-xs bg-gradient-to-r from-amber-500 to-orange-500 border-0">
                  <Sparkles className="h-3 w-3 mr-1" />
                  默认
                </Badge>
              )}
              {!pool.enabled && (
                <Badge variant="destructive" className="text-xs">
                  已禁用
                </Badge>
              )}
            </div>
            <div className="flex items-center gap-4 text-sm text-muted-foreground">
              <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-muted">
                <SchedulingModeIcon className="h-3 w-3" />
                {schedulingModeLabel}
              </span>
              <span className="flex items-center gap-1.5">
                <Key className="h-3 w-3" />
                {pool.availableCredentials}/{pool.totalCredentials} 可用
              </span>
              {pool.hasProxy && (
                <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-600 dark:text-blue-400">
                  <Wifi className="h-3 w-3" />
                  代理
                </span>
              )}
              {pool.sessionCacheSize > 0 && (
                <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-primary/10 text-primary dark:text-primary-foreground">
                  <Users className="h-3 w-3" />
                  {pool.sessionCacheSize} 会话
                </span>
              )}
            </div>
          </div>
        </div>

        <div
          className="flex items-center gap-2"
          onClick={(e) => e.stopPropagation()}
        >
          <Button
            variant="ghost"
            size="sm"
            className="rounded-lg"
            onClick={onEdit}
          >
            编辑
          </Button>
          {!isDefault && (
            <Button
              variant="ghost"
              size="sm"
              className="rounded-lg"
              onClick={() => onToggleEnabled(!pool.enabled)}
            >
              {pool.enabled ? "禁用" : "启用"}
            </Button>
          )}
          {!isDefault && pool.totalCredentials === 0 && (
            <Button
              variant="ghost"
              size="sm"
              className="rounded-lg text-destructive hover:text-destructive"
              onClick={onDelete}
            >
              删除
            </Button>
          )}
        </div>
      </div>

      {expanded && (
        <div className="border-t bg-gradient-to-b from-muted/50 to-muted/20 px-5 py-5">
          {/* 凭证列表头部 */}
          <div className="flex items-center justify-between mb-5">
            <div className="flex items-center gap-2">
              <Shield className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm font-medium">凭据列表</span>
              <Badge variant="secondary" className="text-xs">
                {isDefault ? credentials.length : pool.totalCredentials} 个
              </Badge>
            </div>
            {isDefault && (
              <div className="flex gap-2">
                <Button
                  onClick={onImportCredentials}
                  size="sm"
                  variant="outline"
                  className="rounded-lg"
                >
                  <Upload className="h-4 w-4 mr-1.5" />
                  导入
                </Button>
                <Button
                  onClick={onAddCredential}
                  size="sm"
                  className="rounded-lg bg-primary hover:bg-primary/90"
                >
                  <Plus className="h-4 w-4 mr-1.5" />
                  添加
                </Button>
              </div>
            )}
          </div>

          {/* 加载中状态 */}
          {!isDefault && loadingCredentials && (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              <span className="ml-2 text-muted-foreground">
                加载凭证列表...
              </span>
            </div>
          )}

          {/* 凭证列表 */}
          {(isDefault || (!loadingCredentials && credentialsLoaded)) && (
            <>
              {displayCredentials.length === 0 ? (
                <div className="text-center py-12 rounded-xl border-2 border-dashed border-muted-foreground/20">
                  <Key className="h-12 w-12 mx-auto mb-3 text-muted-foreground/40" />
                  <p className="text-muted-foreground mb-1">暂无凭据</p>
                  <p className="text-sm text-muted-foreground/70">
                    {isDefault
                      ? '点击"添加"或"导入"添加凭据'
                      : "将凭据从其他池转移到此池"}
                  </p>
                </div>
              ) : (
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  {displayCredentials.map((credential) => (
                    <div key={credential.id} className="relative">
                      <CredentialCard
                        credential={credential}
                        onViewBalance={onViewBalance}
                        schedulingMode={pool.schedulingMode as SchedulingMode}
                      />
                      {/* 转移凭证控件 */}
                      {targetPools.length > 0 && onTransferCredential && (
                        <div className="mt-2 flex items-center gap-2 p-2 bg-muted/50 rounded-lg">
                          <ArrowRightLeft className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                          <Select
                            value={
                              transferringId === credential.id
                                ? selectedTargetPool
                                : ""
                            }
                            onValueChange={(value) => {
                              setTransferringId(credential.id);
                              setSelectedTargetPool(value);
                            }}
                          >
                            <SelectTrigger className="h-8 text-xs flex-1">
                              <SelectValue placeholder="转移到..." />
                            </SelectTrigger>
                            <SelectContent>
                              {targetPools.map((p) => (
                                <SelectItem key={p.id} value={p.id}>
                                  {p.name} ({p.id})
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                          <Button
                            size="sm"
                            variant="outline"
                            className="h-8 px-2"
                            disabled={
                              transferringId !== credential.id ||
                              !selectedTargetPool ||
                              transferringId === null
                            }
                            onClick={() => handleTransfer(credential.id)}
                          >
                            {transferringId === credential.id &&
                            selectedTargetPool ? (
                              <Loader2 className="h-3 w-3 animate-spin" />
                            ) : (
                              "转移"
                            )}
                          </Button>
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </>
          )}

          {/* 非默认池的统计信息 */}
          {!isDefault &&
            !loadingCredentials &&
            credentialsLoaded &&
            displayCredentials.length > 0 && (
              <div className="mt-6 pt-4 border-t">
                <div className="grid grid-cols-3 gap-4 max-w-md">
                  <div className="text-center p-3 rounded-lg bg-background shadow-sm">
                    <div className="text-2xl font-bold">
                      {pool.totalCredentials}
                    </div>
                    <div className="text-xs text-muted-foreground">总凭据</div>
                  </div>
                  <div className="text-center p-3 rounded-lg bg-background shadow-sm">
                    <div className="text-2xl font-bold text-green-600">
                      {pool.availableCredentials}
                    </div>
                    <div className="text-xs text-muted-foreground">可用</div>
                  </div>
                  <div className="text-center p-3 rounded-lg bg-background shadow-sm">
                    <div className="text-2xl font-bold text-primary">
                      {pool.sessionCacheSize}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      会话缓存
                    </div>
                  </div>
                </div>
              </div>
            )}
        </div>
      )}
    </Card>
  );
}
