import { useRequirementsStatus } from '@/hooks/useRequirementsStatus';
import { Loader2, CheckCircle2, XCircle, Sparkles } from 'lucide-react';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  TooltipProvider,
} from '@/components/ui/tooltip';
import { useTranslation } from 'react-i18next';

interface AIGenerationIndicatorProps {
  projectId: string;
}

export function AIGenerationIndicator({ projectId }: AIGenerationIndicatorProps) {
  const { t } = useTranslation('projects');
  const { status, isInProgress, isCompleted, isFailed } = useRequirementsStatus(projectId);

  // Don't render anything if no requirements status
  if (!status) {
    return null;
  }

  const getTooltipContent = () => {
    if (isInProgress) {
      switch (status.generation_status) {
        case 'pending':
          return t('aiGeneration.pending');
        case 'analyzing':
          return t('aiGeneration.analyzing');
        case 'generating':
          return t('aiGeneration.inProgress');
        default:
          return t('aiGeneration.inProgress');
      }
    }
    if (isCompleted) {
      const count = status.tasks_generated ?? status.analysis_result?.features.length ?? 0;
      return t('aiGeneration.completed', { count });
    }
    if (isFailed) {
      return status.error_message || t('aiGeneration.failed');
    }
    return '';
  };

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div
            className="flex items-center gap-1 px-1.5 py-0.5 rounded text-xs"
            onClick={(e) => e.stopPropagation()}
          >
            {isInProgress && (
              <>
                <Sparkles className="h-3.5 w-3.5 text-primary" />
                <Loader2 className="h-3 w-3 animate-spin text-primary" />
              </>
            )}
            {isCompleted && (
              <CheckCircle2 className="h-4 w-4 text-green-500" />
            )}
            {isFailed && (
              <XCircle className="h-4 w-4 text-destructive" />
            )}
          </div>
        </TooltipTrigger>
        <TooltipContent>
          <p>{getTooltipContent()}</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
