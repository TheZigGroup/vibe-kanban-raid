import { GitMerge, Loader2 } from 'lucide-react';
import { useReviewAutomation } from '@/hooks/useReviewAutomation';
import { Switch } from '@/components/ui/switch';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  TooltipProvider,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';

interface ReviewAutomationToggleProps {
  projectId: string;
  className?: string;
}

export function ReviewAutomationToggle({
  projectId,
  className,
}: ReviewAutomationToggleProps) {
  const {
    isLoading,
    isEnabled,
    enable,
    disable,
    isEnabling,
    isDisabling,
  } = useReviewAutomation(projectId);

  const isToggling = isEnabling || isDisabling;

  const handleToggle = async (checked: boolean) => {
    if (checked) {
      await enable();
    } else {
      await disable();
    }
  };

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-2', className)}>
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <TooltipProvider>
      <div className={cn('flex items-center gap-3', className)}>
        <Tooltip>
          <TooltipTrigger asChild>
            <div className="flex items-center gap-2">
              <GitMerge
                className={cn(
                  'h-4 w-4',
                  isEnabled ? 'text-primary' : 'text-muted-foreground'
                )}
              />
              <span className="text-sm text-muted-foreground">Auto-merge</span>
              <Switch
                checked={isEnabled}
                onCheckedChange={handleToggle}
                disabled={isToggling}
                aria-label="Toggle review automation"
              />
            </div>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            <p>
              {isEnabled
                ? 'Automatically runs tests and merges tasks in review'
                : 'Enable to automatically test and merge reviewed tasks'}
            </p>
          </TooltipContent>
        </Tooltip>
      </div>
    </TooltipProvider>
  );
}
