import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { requirementsApi } from '@/lib/api';
import type { ProjectRequirementsStatus, CreateProjectRequirements } from 'shared/types';

export function useProjectRequirements(projectId: string | undefined) {
  const queryClient = useQueryClient();

  const query = useQuery<ProjectRequirementsStatus | null>({
    queryKey: ['project-requirements', projectId],
    queryFn: () => (projectId ? requirementsApi.get(projectId) : Promise.resolve(null)),
    enabled: !!projectId,
    refetchInterval: (data) => {
      // Polling while generation is in progress
      const status = data.state.data?.generation_status;
      if (status === 'analyzing' || status === 'generating' || status === 'pending') {
        return 2000; // Poll every 2 seconds
      }
      return false;
    },
  });

  const createMutation = useMutation({
    mutationFn: (data: CreateProjectRequirements) => {
      if (!projectId) throw new Error('Project ID is required');
      return requirementsApi.create(projectId, data);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-requirements', projectId] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => {
      if (!projectId) throw new Error('Project ID is required');
      return requirementsApi.delete(projectId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-requirements', projectId] });
    },
  });

  return {
    requirements: query.data,
    isLoading: query.isLoading,
    error: query.error,
    createRequirements: createMutation.mutateAsync,
    isCreating: createMutation.isPending,
    deleteRequirements: deleteMutation.mutateAsync,
    isDeleting: deleteMutation.isPending,
    refetch: query.refetch,
  };
}
