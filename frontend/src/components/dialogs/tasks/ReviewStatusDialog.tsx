import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { defineModal } from '@/lib/modals';
import { useTranslation } from 'react-i18next';
import { useQuery } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { reviewAutomationApi } from '@/lib/api';
import {
  CheckCircle,
  XCircle,
  AlertTriangle,
  GitMerge,
  SkipForward,
  RefreshCw,
  Clock,
} from 'lucide-react';
import type { ReviewAutomationLog, ReviewAction } from 'shared/types';
import { cn } from '@/lib/utils';

export interface ReviewStatusDialogProps {
  taskId: string;
  taskTitle: string;
}

function getActionIcon(action: ReviewAction) {
  switch (action) {
    case 'test_passed':
      return <CheckCircle className="h-4 w-4 text-green-500" />;
    case 'test_failed':
      return <XCircle className="h-4 w-4 text-red-500" />;
    case 'merge_completed':
      return <GitMerge className="h-4 w-4 text-green-500" />;
    case 'merge_conflict':
      return <AlertTriangle className="h-4 w-4 text-yellow-500" />;
    case 'skipped':
      return <SkipForward className="h-4 w-4 text-muted-foreground" />;
    case 'error':
      return <XCircle className="h-4 w-4 text-red-500" />;
    default:
      return <Clock className="h-4 w-4 text-muted-foreground" />;
  }
}

function getActionLabel(action: ReviewAction): string {
  switch (action) {
    case 'test_passed':
      return 'Tests Passed';
    case 'test_failed':
      return 'Tests Failed';
    case 'merge_completed':
      return 'Merge Completed';
    case 'merge_conflict':
      return 'Merge Conflict';
    case 'skipped':
      return 'Skipped';
    case 'error':
      return 'Error';
    default:
      return action;
  }
}

function getActionBgClass(action: ReviewAction): string {
  switch (action) {
    case 'test_passed':
    case 'merge_completed':
      return 'bg-green-500/10';
    case 'test_failed':
    case 'error':
      return 'bg-red-500/10';
    case 'merge_conflict':
      return 'bg-yellow-500/10';
    default:
      return 'bg-muted';
  }
}

function formatDate(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

const ReviewStatusDialogImpl = NiceModal.create<ReviewStatusDialogProps>(
  ({ taskId, taskTitle }) => {
    const modal = useModal();
    const { t } = useTranslation('tasks');

    const {
      data: logs,
      isLoading,
      isError,
      refetch,
    } = useQuery({
      queryKey: ['reviewLogs', taskId],
      queryFn: () => reviewAutomationApi.getLogsByTask(taskId),
    });

    const handleOpenChange = (open: boolean) => {
      if (!open) {
        modal.hide();
      }
    };

    // Get the latest status for a summary
    const latestLog = logs?.[0];
    const hasConflict = logs?.some((log) => log.action === 'merge_conflict');
    const hasError = logs?.some((log) => log.action === 'error');
    const hasTestFailure = logs?.some((log) => log.action === 'test_failed');

    return (
      <Dialog
        open={modal.visible}
        onOpenChange={handleOpenChange}
        className="max-w-2xl w-[90vw]"
      >
        <DialogContent
          className="p-0 min-w-0"
          onKeyDownCapture={(e) => {
            if (e.key === 'Escape') {
              e.stopPropagation();
              modal.hide();
            }
          }}
        >
          <DialogHeader className="px-4 py-3 border-b">
            <DialogTitle className="flex items-center gap-2">
              <span>Review Status</span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => refetch()}
                className="h-6 w-6 p-0"
              >
                <RefreshCw className="h-3 w-3" />
              </Button>
            </DialogTitle>
            <p className="text-sm text-muted-foreground truncate">{taskTitle}</p>
          </DialogHeader>

          <div className="p-4 max-h-[70vh] overflow-auto space-y-4">
            {/* Summary Section */}
            {latestLog && (
              <div
                className={cn(
                  'p-3 rounded-md border',
                  hasConflict || hasError || hasTestFailure
                    ? 'border-yellow-500/50 bg-yellow-500/10'
                    : 'border-green-500/50 bg-green-500/10'
                )}
              >
                <div className="flex items-center gap-2 font-medium">
                  {hasConflict ? (
                    <>
                      <AlertTriangle className="h-4 w-4 text-yellow-500" />
                      <span>Merge Conflict Detected</span>
                    </>
                  ) : hasTestFailure ? (
                    <>
                      <XCircle className="h-4 w-4 text-red-500" />
                      <span>Tests Failed</span>
                    </>
                  ) : hasError ? (
                    <>
                      <XCircle className="h-4 w-4 text-red-500" />
                      <span>Error During Review</span>
                    </>
                  ) : latestLog.action === 'merge_completed' ? (
                    <>
                      <CheckCircle className="h-4 w-4 text-green-500" />
                      <span>Successfully Merged</span>
                    </>
                  ) : (
                    <>
                      <Clock className="h-4 w-4 text-muted-foreground" />
                      <span>Awaiting Review</span>
                    </>
                  )}
                </div>
                {(hasConflict || hasError) && latestLog.error_message && (
                  <p className="mt-2 text-sm text-muted-foreground">
                    {latestLog.error_message}
                  </p>
                )}
              </div>
            )}

            {isError && (
              <div className="py-8 text-center space-y-3">
                <div className="text-sm text-destructive">
                  Failed to load review logs
                </div>
                <Button variant="outline" size="sm" onClick={() => refetch()}>
                  {t('common:buttons.retry')}
                </Button>
              </div>
            )}

            {isLoading && (
              <div className="py-8 text-center">
                <RefreshCw className="h-6 w-6 animate-spin mx-auto text-muted-foreground" />
                <p className="mt-2 text-sm text-muted-foreground">
                  Loading review logs...
                </p>
              </div>
            )}

            {!isError && !isLoading && logs && logs.length === 0 && (
              <div className="py-8 text-center text-muted-foreground">
                <Clock className="h-8 w-8 mx-auto mb-2 opacity-50" />
                <p className="text-sm">
                  No review automation activity yet.
                </p>
                <p className="text-xs mt-1">
                  Make sure review automation is enabled for this project.
                </p>
              </div>
            )}

            {!isError && !isLoading && logs && logs.length > 0 && (
              <div className="space-y-2">
                <h3 className="text-sm font-medium">Activity Log</h3>
                <div className="space-y-2">
                  {logs.map((log: ReviewAutomationLog) => (
                    <div
                      key={log.id}
                      className={cn(
                        'p-3 rounded-md border',
                        getActionBgClass(log.action)
                      )}
                    >
                      <div className="flex items-center justify-between gap-2">
                        <div className="flex items-center gap-2">
                          {getActionIcon(log.action)}
                          <span className="font-medium text-sm">
                            {getActionLabel(log.action)}
                          </span>
                        </div>
                        <span className="text-xs text-muted-foreground">
                          {formatDate(log.created_at)}
                        </span>
                      </div>
                      {log.error_message && (
                        <p className="mt-2 text-sm text-muted-foreground bg-background/50 p-2 rounded">
                          {log.error_message}
                        </p>
                      )}
                      {log.output && (
                        <details className="mt-2">
                          <summary className="text-xs text-muted-foreground cursor-pointer hover:text-foreground">
                            View output
                          </summary>
                          <pre className="mt-2 text-xs bg-background/50 p-2 rounded overflow-x-auto max-h-48">
                            {log.output}
                          </pre>
                        </details>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </DialogContent>
      </Dialog>
    );
  }
);

export const ReviewStatusDialog = defineModal<ReviewStatusDialogProps, void>(
  ReviewStatusDialogImpl
);
