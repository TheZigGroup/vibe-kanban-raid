import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { agentActivityApi } from '@/lib/api';
import type { AgentActivityStatus, AgentTriggerResponse } from 'shared/types';

export interface UseAgentActivityResult {
  status: AgentActivityStatus | null;
  isLoading: boolean;
  error: Error | null;
  isEnabled: boolean;
  enable: () => Promise<void>;
  disable: () => Promise<void>;
  trigger: () => Promise<AgentTriggerResponse>;
  isEnabling: boolean;
  isDisabling: boolean;
  isTriggering: boolean;
}

export function useAgentActivity(
  projectId: string | undefined
): UseAgentActivityResult {
  const queryClient = useQueryClient();
  const queryKey = ['agent-activity', projectId];

  const query = useQuery<AgentActivityStatus | null>({
    queryKey,
    queryFn: () =>
      projectId ? agentActivityApi.getStatus(projectId) : Promise.resolve(null),
    enabled: !!projectId,
  });

  const enableMutation = useMutation({
    mutationFn: async () => {
      if (!projectId) throw new Error('Project ID is required');
      return agentActivityApi.enable(projectId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey });
    },
  });

  const disableMutation = useMutation({
    mutationFn: async () => {
      if (!projectId) throw new Error('Project ID is required');
      return agentActivityApi.disable(projectId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey });
    },
  });

  const triggerMutation = useMutation({
    mutationFn: async () => {
      if (!projectId) throw new Error('Project ID is required');
      return agentActivityApi.trigger(projectId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey });
      // Also invalidate tasks since a task may have been moved to in-progress
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    },
  });

  return {
    status: query.data ?? null,
    isLoading: query.isLoading,
    error: query.error,
    isEnabled: query.data?.enabled ?? false,
    enable: async () => {
      await enableMutation.mutateAsync();
    },
    disable: async () => {
      await disableMutation.mutateAsync();
    },
    trigger: async () => {
      return triggerMutation.mutateAsync();
    },
    isEnabling: enableMutation.isPending,
    isDisabling: disableMutation.isPending,
    isTriggering: triggerMutation.isPending,
  };
}
