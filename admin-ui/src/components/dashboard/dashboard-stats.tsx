import {
  Database,
  CheckCircle2,
  Key,
  Users,
  RotateCcw,
  TrendingUp,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { Card, CardContent } from "@/components/ui/card";

export interface DashboardStatsData {
  totalPools: number;
  enabledPools: number;
  totalCredentials: number;
  availableCredentials: number;
  sessionCacheSize: number;
  roundRobinCounter: number;
}

export interface DashboardStatsProps {
  stats: DashboardStatsData;
}

export function DashboardStats({ stats }: DashboardStatsProps) {
  const { t } = useTranslation();
  const {
    totalPools,
    enabledPools,
    totalCredentials,
    availableCredentials,
    sessionCacheSize,
    roundRobinCounter,
  } = stats;

  const statItems = [
    {
      icon: Database,
      label: t('dashboard.stats.totalPools'),
      value: totalPools,
      color: "from-cyan-500 to-blue-500",
      bgColor: "bg-cyan-500/10",
      textColor: "text-cyan-600 dark:text-cyan-400",
    },
    {
      icon: CheckCircle2,
      label: t('dashboard.stats.enabledPools'),
      value: enabledPools,
      color: "from-green-500 to-emerald-500",
      bgColor: "bg-green-500/10",
      textColor: "text-green-600 dark:text-green-400",
    },
    {
      icon: Key,
      label: t('dashboard.stats.totalCredentials'),
      value: totalCredentials,
      color: "from-orange-500 to-amber-500",
      bgColor: "bg-orange-500/10",
      textColor: "text-orange-600 dark:text-orange-400",
    },
    {
      icon: TrendingUp,
      label: t('dashboard.stats.availableCredentials'),
      value: availableCredentials,
      color: "from-blue-500 to-cyan-500",
      bgColor: "bg-blue-500/10",
      textColor: "text-blue-600 dark:text-blue-400",
    },
    {
      icon: Users,
      label: t('dashboard.stats.sessionCacheSize'),
      value: sessionCacheSize,
      color: "from-primary/80 to-primary",
      bgColor: "bg-primary/10",
      textColor: "text-primary",
    },
    {
      icon: RotateCcw,
      label: t('dashboard.stats.roundRobinCounter'),
      value: roundRobinCounter,
      color: "from-amber-500 to-yellow-500",
      bgColor: "bg-amber-500/10",
      textColor: "text-amber-600 dark:text-amber-400",
    },
  ];

  return (
    <div className="grid gap-4 grid-cols-2 md:grid-cols-3 lg:grid-cols-6 mb-8">
      {statItems.map((item) => (
        <Card
          key={item.label}
          className="relative overflow-hidden border-0 shadow-sm hover:shadow-md transition-all duration-300 group"
        >
          <div
            className={`absolute inset-0 bg-gradient-to-br ${item.color} opacity-[0.03] group-hover:opacity-[0.06] transition-opacity`}
          />
          <CardContent className="p-4">
            <div className="flex items-start justify-between mb-3">
              <div className={`p-2 rounded-lg ${item.bgColor}`}>
                <item.icon className={`h-4 w-4 ${item.textColor}`} />
              </div>
            </div>
            <div className={`text-2xl font-bold ${item.textColor} mb-1`}>
              {item.value.toLocaleString()}
            </div>
            <div className="text-xs font-medium text-muted-foreground">
              {item.label}
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
