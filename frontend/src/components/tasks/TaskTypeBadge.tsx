import type { TaskType, TaskLayer } from 'shared/types';
import { cn } from '@/lib/utils';

interface TaskTypeBadgeProps {
  taskType: TaskType | null;
  className?: string;
}

const taskTypeConfig: Record<TaskType, { label: string; className: string }> = {
  architecture: {
    label: 'Architecture',
    className: 'bg-indigo-100 text-indigo-700 border-indigo-200',
  },
  mock: {
    label: 'Mock',
    className: 'bg-amber-100 text-amber-700 border-amber-200',
  },
  implementation: {
    label: 'Implementation',
    className: 'bg-emerald-100 text-emerald-700 border-emerald-200',
  },
  integration: {
    label: 'Integration',
    className: 'bg-purple-100 text-purple-700 border-purple-200',
  },
};

export function TaskTypeBadge({ taskType, className }: TaskTypeBadgeProps) {
  if (!taskType) return null;

  const config = taskTypeConfig[taskType];
  if (!config) return null;

  return (
    <span
      className={cn(
        'inline-flex items-center rounded-md border px-1.5 py-0.5 text-xs font-medium',
        config.className,
        className
      )}
    >
      {config.label}
    </span>
  );
}

interface TaskLayerBadgeProps {
  layer: TaskLayer | null;
  className?: string;
}

const layerConfig: Record<TaskLayer, { label: string; className: string }> = {
  frontend: {
    label: 'Frontend',
    className: 'bg-sky-100 text-sky-700 border-sky-200',
  },
  backend: {
    label: 'Backend',
    className: 'bg-orange-100 text-orange-700 border-orange-200',
  },
  data: {
    label: 'Data',
    className: 'bg-cyan-100 text-cyan-700 border-cyan-200',
  },
  fullstack: {
    label: 'Fullstack',
    className: 'bg-violet-100 text-violet-700 border-violet-200',
  },
  devops: {
    label: 'DevOps',
    className: 'bg-slate-100 text-slate-700 border-slate-200',
  },
  testing: {
    label: 'Testing',
    className: 'bg-rose-100 text-rose-700 border-rose-200',
  },
};

export function TaskLayerBadge({ layer, className }: TaskLayerBadgeProps) {
  if (!layer) return null;

  const config = layerConfig[layer];
  if (!config) return null;

  return (
    <span
      className={cn(
        'inline-flex items-center rounded-md border px-1.5 py-0.5 text-xs font-medium',
        config.className,
        className
      )}
    >
      {config.label}
    </span>
  );
}
