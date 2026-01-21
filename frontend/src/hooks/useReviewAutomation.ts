import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { reviewAutomationApi } from '@/lib/api';
import type {
  ReviewAutomationStatus,
  ReviewAutomationLog,
} from 'shared/types';

export interface UseReviewAutomationResult {
  status: ReviewAutomationStatus | null;
  logs: ReviewAutomationLog[];
  isLoading: boolean;
  error: Error | null;
  isEnabled: boolean;
  enable: () => Promise<void>;
  disable: () => Promise<void>;
  isEnabling: boolean;
  isDisabling: boolean;
}

export function useReviewAutomation(
  projectId: string | undefined
): UseReviewAutomationResult {
  const queryClient = useQueryClient();
  const statusQueryKey = ['review-automation', projectId];
  const logsQueryKey = ['review-automation-logs', projectId];

  const statusQuery = useQuery<ReviewAutomationStatus | null>({
    queryKey: statusQueryKey,
    queryFn: () =>
      projectId
        ? reviewAutomationApi.getStatus(projectId)
        : Promise.resolve(null),
    enabled: !!projectId,
  });

  const logsQuery = useQuery<ReviewAutomationLog[]>({
    queryKey: logsQueryKey,
    queryFn: () =>
      projectId ? reviewAutomationApi.getLogs(projectId) : Promise.resolve([]),
    enabled: !!projectId,
  });

  const enableMutation = useMutation({
    mutationFn: async () => {
      if (!projectId) throw new Error('Project ID is required');
      return reviewAutomationApi.enable(projectId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: statusQueryKey });
    },
  });

  const disableMutation = useMutation({
    mutationFn: async () => {
      if (!projectId) throw new Error('Project ID is required');
      return reviewAutomationApi.disable(projectId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: statusQueryKey });
    },
  });

  return {
    status: statusQuery.data ?? null,
    logs: logsQuery.data ?? [],
    isLoading: statusQuery.isLoading,
    error: statusQuery.error,
    isEnabled: statusQuery.data?.enabled ?? false,
    enable: async () => {
      await enableMutation.mutateAsync();
    },
    disable: async () => {
      await disableMutation.mutateAsync();
    },
    isEnabling: enableMutation.isPending,
    isDisabling: disableMutation.isPending,
  };
}
