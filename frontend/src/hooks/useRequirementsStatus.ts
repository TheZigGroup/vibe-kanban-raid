import { useQuery } from '@tanstack/react-query';
import { requirementsApi } from '@/lib/api';
import type { ProjectRequirementsStatus } from 'shared/types';

type Options = {
  enabled?: boolean;
};

export function useRequirementsStatus(projectId?: string, opts?: Options) {
  const enabled = (opts?.enabled ?? true) && !!projectId;

  const query = useQuery<ProjectRequirementsStatus | null>({
    queryKey: ['requirementsStatus', projectId],
    queryFn: () => requirementsApi.get(projectId!),
    enabled,
    // Poll every 2 seconds while generation is in progress
    refetchInterval: (query) => {
      const status = query.state.data?.generation_status;
      if (status === 'pending' || status === 'analyzing' || status === 'generating') {
        return 2000;
      }
      return false; // Stop polling when completed/failed/null
    },
    staleTime: 5000,
  });

  const status = query.data;
  const isInProgress =
    status?.generation_status === 'pending' ||
    status?.generation_status === 'analyzing' ||
    status?.generation_status === 'generating';
  const isCompleted = status?.generation_status === 'completed';
  const isFailed = status?.generation_status === 'failed';

  return {
    status,
    isLoading: query.isLoading,
    isInProgress,
    isCompleted,
    isFailed,
    refetch: query.refetch,
  };
}
