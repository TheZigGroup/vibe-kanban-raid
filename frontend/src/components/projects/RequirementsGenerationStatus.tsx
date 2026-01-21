import { useEffect, useState, useCallback } from 'react';
import { requirementsApi } from '@/lib/api';
import { ProjectRequirementsStatus, GenerationStatus } from 'shared/types';
import { Loader2, CheckCircle2, XCircle, Sparkles, AlertCircle } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';

interface RequirementsGenerationStatusProps {
  projectId: string;
  onComplete?: () => void;
  onError?: (error: string) => void;
}

const STATUS_LABELS: Record<GenerationStatus, string> = {
  pending: 'Waiting to start...',
  analyzing: 'Analyzing requirements...',
  generating: 'Generating tasks...',
  completed: 'Tasks generated successfully!',
  failed: 'Generation failed',
};

const STATUS_DESCRIPTIONS: Record<GenerationStatus, string> = {
  pending: 'Your requirements are queued for analysis',
  analyzing: 'AI is extracting features from your requirements',
  generating: 'Creating tasks for your kanban board',
  completed: 'Your tasks are ready on the kanban board',
  failed: 'Something went wrong during generation',
};

export function RequirementsGenerationStatus({
  projectId,
  onComplete,
  onError,
}: RequirementsGenerationStatusProps) {
  const [status, setStatus] = useState<ProjectRequirementsStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const result = await requirementsApi.get(projectId);
      setStatus(result);
      setError(null);

      if (result?.generation_status === 'completed') {
        onComplete?.();
      } else if (result?.generation_status === 'failed') {
        onError?.(result.error_message || 'Generation failed');
      }

      return result;
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Failed to fetch status';
      setError(errorMessage);
      return null;
    } finally {
      setLoading(false);
    }
  }, [projectId, onComplete, onError]);

  useEffect(() => {
    fetchStatus();

    // Poll for status updates while in progress
    const interval = setInterval(async () => {
      const result = await fetchStatus();
      if (
        result?.generation_status === 'completed' ||
        result?.generation_status === 'failed'
      ) {
        clearInterval(interval);
      }
    }, 2000);

    return () => clearInterval(interval);
  }, [fetchStatus]);

  const handleRetry = async () => {
    setLoading(true);
    setError(null);
    // Delete and let user try again
    try {
      await requirementsApi.delete(projectId);
      setStatus(null);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'Failed to reset requirements'
      );
    } finally {
      setLoading(false);
    }
  };

  if (loading && !status) {
    return (
      <div className="flex items-center gap-2 text-muted-foreground p-4">
        <Loader2 className="h-4 w-4 animate-spin" />
        <span>Loading...</span>
      </div>
    );
  }

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertCircle className="h-4 w-4" />
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (!status) {
    return null;
  }

  const isInProgress =
    status.generation_status === 'pending' ||
    status.generation_status === 'analyzing' ||
    status.generation_status === 'generating';

  const isCompleted = status.generation_status === 'completed';
  const isFailed = status.generation_status === 'failed';

  return (
    <div className="rounded-lg border bg-card p-4">
      <div className="flex items-start gap-3">
        <div className="mt-0.5">
          {isInProgress && (
            <Loader2 className="h-5 w-5 animate-spin text-primary" />
          )}
          {isCompleted && (
            <CheckCircle2 className="h-5 w-5 text-green-500" />
          )}
          {isFailed && <XCircle className="h-5 w-5 text-destructive" />}
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <Sparkles className="h-4 w-4 text-primary" />
            <span className="font-medium">
              {STATUS_LABELS[status.generation_status]}
            </span>
          </div>

          <p className="text-sm text-muted-foreground mt-1">
            {STATUS_DESCRIPTIONS[status.generation_status]}
          </p>

          {status.error_message && (
            <p className="text-sm text-destructive mt-2">
              {status.error_message}
            </p>
          )}

          {status.analysis_result && isCompleted && (
            <div className="mt-3 text-sm">
              <span className="text-muted-foreground">
                {status.analysis_result.features.length} features identified
              </span>
              {status.tasks_generated !== undefined && status.tasks_generated !== null && (
                <span className="text-muted-foreground">
                  {' '}&bull; {status.tasks_generated} tasks created
                </span>
              )}
            </div>
          )}

          {isFailed && (
            <Button
              variant="outline"
              size="sm"
              className="mt-3"
              onClick={handleRetry}
            >
              Try Again
            </Button>
          )}
        </div>
      </div>

      {isInProgress && (
        <div className="mt-4">
          <div className="h-1.5 bg-muted rounded-full overflow-hidden">
            <div
              className="h-full bg-primary rounded-full animate-pulse"
              style={{
                width:
                  status.generation_status === 'pending'
                    ? '10%'
                    : status.generation_status === 'analyzing'
                      ? '40%'
                      : '70%',
                transition: 'width 0.5s ease-in-out',
              }}
            />
          </div>
        </div>
      )}
    </div>
  );
}
